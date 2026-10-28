use std::sync::Once;

use lucent_host::ScriptLink;

use crate::{RuntimeError, host};

type Rejections = Vec<(v8::Global<v8::Promise>, String)>;

pub(crate) struct Engine {
    context: v8::Global<v8::Context>,
    isolate: v8::OwnedIsolate,
}

impl Engine {
    pub(crate) fn new(link: ScriptLink) -> Result<Self, RuntimeError> {
        static PLATFORM: Once = Once::new();
        PLATFORM.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
        let mut isolate = v8::Isolate::new(Default::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        isolate.set_slot(link);
        isolate.set_slot(Rejections::new());
        isolate.set_promise_reject_callback(promise_rejected);
        let context = {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            let host = v8::Object::new(scope);
            for (name, function) in [
                ("swf", v8::Function::new(scope, host_swf)),
                ("read", v8::Function::new(scope, host_read)),
            ] {
                let key = v8::String::new(scope, name).unwrap();
                let function = function.ok_or_else(|| {
                    RuntimeError::JavaScript("couldn't create host function".into())
                })?;
                host.set(scope, key.into(), function.into());
            }
            let key = v8::String::new(scope, "__host").unwrap();
            context.global(scope).set(scope, key.into(), host.into());
            v8::Global::new(scope, context)
        };
        Ok(Self { context, isolate })
    }

    pub(crate) fn eval(&mut self, name: &str, source: &str) -> Result<String, RuntimeError> {
        v8::scope!(let scope, &mut self.isolate);
        let context = v8::Local::new(scope, &self.context);
        let scope = &mut v8::ContextScope::new(scope, context);
        v8::tc_scope!(let tc, scope);
        let result = (|| {
            let source = v8::String::new(tc, source)
                .ok_or_else(|| RuntimeError::JavaScript("script too large".into()))?;
            let name = v8::String::new(tc, name)
                .ok_or_else(|| RuntimeError::JavaScript("script name too large".into()))?;
            let origin = v8::ScriptOrigin::new(
                tc,
                name.into(),
                0,
                0,
                false,
                0,
                None,
                false,
                false,
                false,
                None,
            );
            let script =
                v8::Script::compile(tc, source, Some(&origin)).ok_or_else(|| exception_text(tc))?;
            script.run(tc).ok_or_else(|| exception_text(tc))
        })();
        tc.perform_microtask_checkpoint();
        let result = result.and_then(|value| {
            if tc.has_caught() {
                return Err(exception_text(tc));
            }
            let json = v8::json::stringify(tc, value).ok_or_else(|| exception_text(tc))?;
            let json = json.to_rust_string_lossy(tc);
            Ok(if json == "undefined" {
                "null".into()
            } else {
                json
            })
        });
        tc.perform_microtask_checkpoint();
        let rejections = tc.get_slot_mut::<Rejections>().unwrap();
        let errors: Vec<_> = rejections.drain(..).map(|(_, error)| error).collect();
        let json = result?;
        if tc.has_caught() {
            return Err(exception_text(tc));
        }
        if !errors.is_empty() {
            return Err(RuntimeError::JavaScript(errors.join("\n")));
        }
        Ok(json)
    }
}

fn exception_text(
    scope: &mut v8::PinnedRef<'_, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
) -> RuntimeError {
    RuntimeError::JavaScript(
        scope
            .stack_trace()
            .or_else(|| scope.exception())
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "JavaScript operation failed".into()),
    )
}

extern "C" fn promise_rejected(message: v8::PromiseRejectMessage) {
    v8::callback_scope!(unsafe scope, &message);
    v8::scope!(let scope, scope);
    let promise = v8::Global::new(scope, message.get_promise());
    match message.get_event() {
        v8::PromiseRejectEvent::PromiseRejectWithNoHandler => {
            let Some(value) = message.get_value() else {
                return;
            };
            let stack = v8::Local::<v8::Object>::try_from(value)
                .ok()
                .and_then(|object| {
                    let key = v8::String::new(scope, "stack")?;
                    object
                        .get(scope, key.into())
                        .filter(|value| value.is_string())
                })
                .unwrap_or(value)
                .to_rust_string_lossy(scope);
            scope
                .get_slot_mut::<Rejections>()
                .unwrap()
                .push((promise, stack));
        }
        v8::PromiseRejectEvent::PromiseHandlerAddedAfterReject => {
            scope
                .get_slot_mut::<Rejections>()
                .unwrap()
                .retain(|(pending, _)| *pending != promise);
        }
        _ => {}
    }
}

fn return_reply(
    scope: &mut v8::PinScope,
    mut reply: v8::ReturnValue<v8::Value>,
    result: Result<Option<String>, String>,
) {
    match result {
        Ok(None) => {}
        Ok(Some(json)) => {
            if let Some(value) = v8::String::new(scope, &json) {
                reply.set(value.into());
            } else {
                throw(scope, "host reply too large");
            }
        }
        Err(message) => throw(scope, &message),
    }
}

fn throw(scope: &mut v8::PinScope, message: &str) {
    if let Some(message) = v8::String::new(scope, message) {
        let error = v8::Exception::error(scope, message);
        scope.throw_exception(error);
    }
}

fn host_swf(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    reply: v8::ReturnValue<v8::Value>,
) {
    let Some(method) = args.get(0).to_string(scope) else {
        return;
    };
    let method = method.to_rust_string_lossy(scope);
    let Some(json) = args.get(1).to_string(scope) else {
        return;
    };
    let json = json.to_rust_string_lossy(scope);
    let result = host::swf(scope.get_slot::<ScriptLink>().unwrap(), method, &json);
    return_reply(scope, reply, result);
}

fn host_read(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    reply: v8::ReturnValue<v8::Value>,
) {
    let Some(json) = args.get(0).to_string(scope) else {
        return;
    };
    let json = json.to_rust_string_lossy(scope);
    let result = host::read(scope.get_slot::<ScriptLink>().unwrap(), &json).map(Some);
    return_reply(scope, reply, result);
}
