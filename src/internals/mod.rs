use crate::prelude::*;
use futures_util::FutureExt;

/// Paramter type info (meant to be used in attribute macro).
#[derive(Debug, Clone, Copy)]
pub enum ParamType {
    String,
    Int,
    Bool,
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Int(isize),
    Bool(bool),
}

pub(crate) type HandlerFn =
    fn(
        MessageData,
        Vec<Value>,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = ()> + Send + 'static>>;

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub custom_prefix: bool,
    pub args: Vec<ParamType>,
    pub handler_fn: HandlerFn,
}

impl Command {
    pub async fn call(&self, data: MessageData) {
        let split = data.content.split_whitespace().collect::<Vec<_>>();

        // -1 because of the command name
        assert_eq!(split.len() - 1, self.args.len());

        // check if this command is really called
        assert_eq!(split[0], self.name);

        let mut args: Vec<Value> = Vec::with_capacity(self.args.len());

        for (idx, ty) in self.args.iter().enumerate() {
            match ty {
                ParamType::String => args.push(Value::String((split[idx + 1].to_owned()))),
                ParamType::Int => args.push(Value::Int(split[idx + 1].parse::<isize>().unwrap())),
                ParamType::Bool => args.push(Value::Bool(split[idx + 1].parse::<bool>().unwrap())),

                _ => {}
            }
        }

        let fut = ((self.handler_fn)(data, args));
        let boxed_fut: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> =
            Box::pin(fut);
        boxed_fut.await;
    }
}