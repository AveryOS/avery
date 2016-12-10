
// Usermode objects

type FutureId; // Refers to the kernel object Call with Future capability.
type PromiseId; // Refers to the kernel object Call with Promise capability.

type CapId = usize; // Identifies a cap created in a Vat

struct LocalCap {
  vtable: &'static [fn()],
  rc: usize,
  // ...
}

struct RemoteCapId(VatId, CapId) {}

struct Payload {
  bytes: Vec<u8>,
  cap: Option<RemoteCapId>,
}

enum Cap {
  Local(Rc<LocalCap>),
  Remote(RemoteCapId),
  Future(CallId),
}

impl Cap {
  fn call(&self, payload: Payload) -> Cap {
    vat.call(self, payload)
    Cap::Future()
  }
}

// Trusted kernel objects

struct Call {
  state: CallState,
  promises: usize,
  promise_access: HashSet<VatId>,
  futures: usize,
  future_access: HashSet<VatId>,
}

enum CallState {
  Unresolved(Vec<Message>),
  Resolved(RemoteCapId),
}

enum Message {
  Call(CapId, Payload),
  Reply(CallId, Payload),
  Resolve(CallId, RemoteCapId),
  Release(VatId, CapId), // Release CapId reference from VatId
  Provide(VatId, CapId), // Give access to the local cap CapId to the vat VatId
}

struct Vat {
  id: VatId,
  inbox: Vec<Message>,
  promises: HashMap<PromiseId, Call>,
  futures: HashMap<FutureId, Call>,
  connections: HashMap<Rc<Vat>, Connection>, 
}

impl Vat {
  fn process(&mut self) -> .. {


  }

  fn reply(&mut self, promise: PromiseId, result: RemoteCapId) {
    let call = get_call_object(promise);
    assert!(call.promise_access.contains(self.id));
    if let CallState::Unresolved(queue) = call.state {
      for msg in queue {
        // send msg to result's vat
      }
    } else {
      panic!();
    }
    call.state = CallState::Resolved(result);
    for vat in call.future_access {
      // send Resolve to vat
    }
  }

  fn pipeline(&mut self, future: FutureId) {

  }

  fn call(&mut self, cap: RemoteCapId) {

  }
}

static Vats: HashMap<VatId, Rc<Vat>>;

struct Connection {
  exports: HashMap<Rc<LocalCap>, usize> // Map from a local cap to a count of references in the other end of the connection
}
