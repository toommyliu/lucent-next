use std::ffi::c_void;
use std::sync::{Arc, Once, mpsc};
use std::time::{Duration, Instant};

use lucent_host::ScriptLink;
use lucent_host::bridge::{Error as EventError, ScriptEvent};

use crate::interrupt::Interrupt;
use crate::{Kill, RuntimeError, host};

const HEAP_LIMIT: usize = 64 << 20;

type Rejections = Vec<(v8::Global<v8::Promise>, v8::Global<v8::Value>)>;

type EvalReply = mpsc::Sender<Result<String, RuntimeError>>;

enum Completion {
    Value(String),
    Promise(v8::Global<v8::Promise>),
}

struct ActiveEval {
    promise: v8::Global<v8::Promise>,
    reply: EvalReply,
}

pub(crate) struct Engine {
    active: Vec<ActiveEval>,
    context: v8::Global<v8::Context>,
    isolate: v8::OwnedIsolate,
}

impl Engine {
    pub(crate) fn new(link: ScriptLink, interrupt: Arc<Interrupt>) -> Result<Self, RuntimeError> {
        static PLATFORM: Once = Once::new();
        PLATFORM.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
        let mut isolate = v8::Isolate::new(v8::CreateParams::default().heap_limits(0, HEAP_LIMIT));
        isolate.add_near_heap_limit_callback(
            near_heap_limit,
            Arc::as_ptr(&interrupt).cast_mut().cast(),
        );
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        isolate.set_slot(interrupt.clone());
        isolate.set_slot(link);
        isolate.set_slot(Instant::now());
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
                ("command", v8::Function::new(scope, host_command)),
                ("now", v8::Function::new(scope, host_now)),
                ("log", v8::Function::new(scope, host_log)),
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
        let mut engine = Self {
            active: Vec::new(),
            context,
            isolate,
        };
        engine.evaluate("runtime.js", include_str!("../../../assets/runtime.js"))?;
        interrupt.attach(engine.isolate.thread_safe_handle());
        Ok(engine)
    }

    pub(crate) fn kill(mut self, kill: Kill) -> ScriptLink {
        self.isolate.get_slot::<Arc<Interrupt>>().unwrap().detach();
        for active in self.active.drain(..) {
            let _ = active.reply.send(Err(RuntimeError::Killed(kill)));
        }
        self.isolate.remove_slot::<ScriptLink>().unwrap()
    }

    pub(crate) fn try_event(&self) -> Result<Option<ScriptEvent>, EventError> {
        self.isolate.get_slot::<ScriptLink>().unwrap().try_event()
    }

    fn overflowed(&self) -> bool {
        self.isolate.get_slot::<ScriptLink>().unwrap().overflowed()
    }

    pub(crate) fn events_lost(&mut self) {
        for active in self.active.drain(..) {
            let _ = active.reply.send(Err(RuntimeError::EventsLost));
        }
    }

    pub(crate) fn eval(&mut self, name: &str, source: &str, reply: EvalReply) {
        match self.evaluate(name, source) {
            Ok(Completion::Promise(promise)) => self.active.push(ActiveEval { promise, reply }),
            Ok(Completion::Value(json)) => {
                let _ = reply.send(Ok(json));
            }
            Err(error) => {
                let _ = reply.send(Err(error));
            }
        }
        self.check_pending();
    }

    fn in_context<R>(
        &mut self,
        run: impl FnOnce(
            &mut v8::PinnedRef<'_, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
        ) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        halted(&self.isolate)?;
        let result = {
            v8::scope!(let scope, &mut self.isolate);
            let context = v8::Local::new(scope, &self.context);
            let scope = &mut v8::ContextScope::new(scope, context);
            v8::tc_scope!(let tc, scope);
            run(tc)
        };
        halted(&self.isolate)?;
        result
    }

    fn evaluate(&mut self, name: &str, source: &str) -> Result<Completion, RuntimeError> {
        self.in_context(|tc| {
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
                let script = v8::Script::compile(tc, source, Some(&origin))
                    .ok_or_else(|| exception_text(tc))?;
                script.run(tc).ok_or_else(|| exception_text(tc))
            })();
            let result = result.map(|value| {
                if let Ok(promise) = v8::Local::<v8::Promise>::try_from(value) {
                    promise.mark_as_handled();
                    let promise = v8::Global::new(tc, promise);
                    tc.get_slot_mut::<Rejections>()
                        .unwrap()
                        .retain(|(pending, _)| *pending != promise);
                    (value, Some(promise))
                } else {
                    (value, None)
                }
            });
            checkpoint(tc)?;
            let result = result.and_then(|(value, promise)| {
                if tc.has_caught() {
                    return Err(exception_text(tc));
                }
                match promise {
                    Some(promise) => Ok(Completion::Promise(promise)),
                    None => stringify(tc, value).map(Completion::Value),
                }
            });
            settle(tc, result)
        })
    }

    fn check_pending(&mut self) {
        loop {
            let remaining = self.active.len();
            for active in std::mem::take(&mut self.active) {
                if self.overflowed() {
                    self.active.push(active);
                    continue;
                }
                let settled = self.in_context(|tc| {
                    let promise = v8::Local::new(tc, &active.promise);
                    let result = match promise.state() {
                        v8::PromiseState::Pending => return Ok(None),
                        v8::PromiseState::Fulfilled => stringify(tc, promise.result(tc)),
                        v8::PromiseState::Rejected => {
                            let reason = v8::Global::new(tc, promise.result(tc));
                            Err(RuntimeError::JavaScript(rejection_text(tc, &reason)))
                        }
                    };
                    Ok(Some(settle(tc, result)))
                });
                match settled {
                    Ok(Some(result)) if !self.overflowed() => {
                        let _ = active.reply.send(result);
                    }
                    _ => self.active.push(active),
                }
            }
            if self.active.len() == remaining {
                break;
            }
        }
    }

    pub(crate) fn turn(&mut self, items: &str) -> Option<Instant> {
        let (result, rejections) = self
            .in_context(|tc| {
                let result = (|| {
                    let items = v8::String::new(tc, items).ok_or_else(|| {
                        RuntimeError::JavaScript("script events too large".into())
                    })?;
                    let errors = call_lucent(tc, "turn", &[items.into()])?;
                    if let Ok(errors) = v8::Local::<v8::Array>::try_from(errors) {
                        for index in 0..errors.length() {
                            if let Some(error) = errors.get_index(tc, index) {
                                tracing::error!(
                                    error = error.to_rust_string_lossy(tc),
                                    "script callback failed"
                                );
                            }
                        }
                    }
                    Ok::<_, RuntimeError>(())
                })();
                checkpoint(tc)?;
                Ok((result, take_rejections(tc)))
            })
            .ok()?;
        if let Err(error) = result {
            tracing::error!(%error, "script turn failed");
        }
        for error in rejections {
            tracing::error!(%error, "unhandled script rejection");
        }
        self.check_pending();
        self.wake()
    }

    fn wake(&mut self) -> Option<Instant> {
        let started = *self.isolate.get_slot::<Instant>().unwrap();
        let result = self.in_context(|tc| {
            call_lucent(tc, "wake", &[])
                .and_then(|value| value.number_value(tc).ok_or_else(|| exception_text(tc)))
        });
        let deadline = match result {
            Ok(deadline) if deadline.is_finite() => deadline,
            Ok(_) | Err(RuntimeError::Killed(_) | RuntimeError::Stopped) => return None,
            Err(error) => {
                tracing::error!(%error, "script timer deadline failed");
                return None;
            }
        };
        let delay = Duration::try_from_secs_f64(deadline.max(0.0) / 1000.0).ok()?;
        started.checked_add(delay)
    }
}

fn settle<T>(
    tc: &mut v8::PinnedRef<'_, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
    result: Result<T, RuntimeError>,
) -> Result<T, RuntimeError> {
    checkpoint(tc)?;
    let errors = take_rejections(tc);
    let value = result?;
    if tc.has_caught() {
        return Err(exception_text(tc));
    }
    if !errors.is_empty() {
        return Err(RuntimeError::JavaScript(errors.join("\n")));
    }
    Ok(value)
}

fn stringify(
    scope: &mut v8::PinnedRef<'_, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
    value: v8::Local<v8::Value>,
) -> Result<String, RuntimeError> {
    halted(scope)?;
    let json = v8::json::stringify(scope, value).ok_or_else(|| exception_text(scope))?;
    let json = json.to_rust_string_lossy(scope);
    Ok(if json == "undefined" {
        "null".into()
    } else {
        json
    })
}

fn call_lucent<'s>(
    scope: &mut v8::PinnedRef<'s, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
    method: &str,
    args: &[v8::Local<v8::Value>],
) -> Result<v8::Local<'s, v8::Value>, RuntimeError> {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__lucent").unwrap();
    let object = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        .ok_or_else(|| exception_text(scope))?;
    let key = v8::String::new(scope, method).unwrap();
    let function = object
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        .ok_or_else(|| exception_text(scope))?;
    function
        .call(scope, object.into(), args)
        .ok_or_else(|| exception_text(scope))
}

fn take_rejections(
    scope: &mut v8::PinnedRef<'_, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
) -> Vec<String> {
    let rejections = std::mem::take(scope.get_slot_mut::<Rejections>().unwrap());
    rejections
        .into_iter()
        .map_while(|(_, reason)| {
            halted(scope)
                .is_ok()
                .then(|| rejection_text(scope, &reason))
        })
        .collect()
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

fn halted(isolate: &v8::Isolate) -> Result<(), RuntimeError> {
    match isolate.get_slot::<Arc<Interrupt>>().unwrap().stop() {
        Some(stop) => Err(stop.into()),
        None => Ok(()),
    }
}

fn checkpoint(
    scope: &mut v8::PinnedRef<'_, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
) -> Result<(), RuntimeError> {
    halted(scope)?;
    scope.perform_microtask_checkpoint();
    Ok(())
}

unsafe extern "C" fn near_heap_limit(data: *mut c_void, current: usize, _: usize) -> usize {
    let interrupt = unsafe { &*data.cast::<Interrupt>() };
    interrupt.kill(Kill::HeapLimit);
    current.saturating_add(HEAP_LIMIT)
}

fn rejection_text(
    scope: &mut v8::PinnedRef<'_, v8::TryCatch<'_, '_, v8::HandleScope<'_>>>,
    value: &v8::Global<v8::Value>,
) -> String {
    let value = v8::Local::new(scope, value);
    v8::Local::<v8::Object>::try_from(value)
        .ok()
        .and_then(|object| {
            let key = v8::String::new(scope, "stack")?;
            object
                .get(scope, key.into())
                .filter(|value| value.is_string())
        })
        .unwrap_or(value)
        .to_rust_string_lossy(scope)
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
            let value = v8::Global::new(scope, value);
            scope
                .get_slot_mut::<Rejections>()
                .unwrap()
                .push((promise, value));
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
    let result = host::swf(
        scope.get_slot::<ScriptLink>().unwrap(),
        scope.get_slot::<Arc<Interrupt>>().unwrap(),
        method,
        &json,
    );
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
    let result = host::read(
        scope.get_slot::<ScriptLink>().unwrap(),
        scope.get_slot::<Arc<Interrupt>>().unwrap(),
        &json,
    )
    .map(Some);
    return_reply(scope, reply, result);
}

fn host_command(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    reply: v8::ReturnValue<v8::Value>,
) {
    let Some(json) = args.get(0).to_string(scope) else {
        return;
    };
    let json = json.to_rust_string_lossy(scope);
    let result = host::command(
        scope.get_slot::<ScriptLink>().unwrap(),
        scope.get_slot::<Arc<Interrupt>>().unwrap(),
        &json,
    )
    .map(Some);
    return_reply(scope, reply, result);
}

fn host_now(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut reply: v8::ReturnValue<v8::Value>,
) {
    reply.set_double(scope.get_slot::<Instant>().unwrap().elapsed().as_secs_f64() * 1000.0);
}

fn host_log(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _reply: v8::ReturnValue<v8::Value>,
) {
    let level = args.get(0).to_rust_string_lossy(scope);
    let text = args.get(1).to_rust_string_lossy(scope);
    match level.as_str() {
        "debug" => tracing::debug!(target: "lucent_runtime::script", "{text}"),
        "warn" => tracing::warn!(target: "lucent_runtime::script", "{text}"),
        "error" => tracing::error!(target: "lucent_runtime::script", "{text}"),
        _ => tracing::info!(target: "lucent_runtime::script", "{text}"),
    }
}
