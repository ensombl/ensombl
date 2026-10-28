//! Minimal CQRS primitives. Every use case is a message type with a single
//! handler; commands may change state, queries must not.

use super::Result;

pub trait Command {
    type Output;
}

pub trait CommandHandler<C: Command> {
    fn handle(&self, command: C) -> Result<C::Output>;
}

pub trait Query {
    type Output;
}

pub trait QueryHandler<Q: Query> {
    fn handle(&self, query: Q) -> Result<Q::Output>;
}
