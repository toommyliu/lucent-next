use ruffle_core::context::UpdateContext;
use ruffle_core::external::{ExternalInterfaceProvider, Value};

pub(crate) struct Bridge;

impl ExternalInterfaceProvider for Bridge {
    fn call_method(&self, _context: &mut UpdateContext<'_>, name: &str, _args: &[Value]) -> Value {
        tracing::debug!(event = name, "loader event");
        Value::Undefined
    }
    fn on_callback_available(&self, _name: &str) {}
    fn get_id(&self) -> Option<String> {
        None
    }
}
