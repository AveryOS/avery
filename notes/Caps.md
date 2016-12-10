# Capability system

This is a sketch of a capability system which utilised data and operations in kernel mode.

There are 2 kernel objects used to support the capability system, `Vat` and `Call`. `Cap` is a global identifier of a capability consisting of a `Vat` reference and a vat defined number.

## Messages

```rust
enum Message {
  Call(Call, Cap, Vec<u8>, Vec<Cap>),
  Reply(Call, Vec<u8>),
  Resolve(Call, Cap),
  Provide(Vat, Cap),
}
```

## The Vat kernel object

Vats contains a queue of messages and a map of vats to sets of capability ids. The map represents which other vats have access and the set which capabilites each vat can access.

```rust
struct Vat {
  inbox: Vec<Message>,
  connections: Map<Vat, Set<Cap>>, 
}
```

User-mode can block on a message from a Vat. However `Provide` messages will not be returned as there nothing for user-mode to do with them.

## The Call kernel object

The call object is created for each remote call. It contains either a queue of messages or a cap which was returned from the call. It also has a set of vats which indicated if each vat has access to it as a future. During a RPC, the caller will gain access to it as a future and the callee will gain access to it as a promise.

```rust
struct Call {
  state: CallState,
  caller: Vat,
  callee: Vat,
  futures: Set<Vat>,
}

enum CallState {
  Unresolved(Vec<Message>),
  Resolved(Cap),
}
```

Vats with future access can queue messages which will be sent when the call is complete.

The callee can reply to the `Call` object causing it to send all it's queued messages and then send a `Resolve` message to each of vats with future access. It will also then send a `Reply` message with the result to the caller.

## Sending capabilities

When we are sending a `Call` cap in a payload, we add access to the receiver in either the `promises` or the `futures` field of `Call` depending what the cap gives access to.

When we are sending a cap belonging to the current vat, we add the cap in the set associated with the receiver in the `connections` map.

When we are sending a cap belonging to another vat to a third vat we send a `Provide` message to the owner which will grant access to the third vat.