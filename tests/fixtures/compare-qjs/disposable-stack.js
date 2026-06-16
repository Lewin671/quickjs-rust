(function () {
  let order = [];
  let stack = new DisposableStack();
  let resource = { [Symbol.dispose]() { order.push("use"); } };
  stack.use(resource);
  stack.adopt("value", value => order.push("adopt:" + value));
  stack.defer(() => order.push("defer"));
  stack.dispose();
  stack.dispose();

  let movedSource = new DisposableStack();
  movedSource.defer(() => order.push("moved:first"));
  movedSource.defer(() => order.push("moved:second"));
  let moved = movedSource.move();
  let moveState = [
    movedSource.disposed,
    moved.disposed,
    moved !== movedSource,
    moved instanceof DisposableStack
  ].join(",");
  moved.dispose();

  let asyncMovedSource = new AsyncDisposableStack();
  asyncMovedSource.defer(() => order.push("async-moved:second"));
  let asyncMoved = asyncMovedSource.move();
  let asyncMoveState = [
    asyncMovedSource.disposed,
    asyncMoved.disposed,
    asyncMoved !== asyncMovedSource,
    asyncMoved instanceof AsyncDisposableStack
  ].join(",");
  asyncMoved.disposeAsync();

  let error1 = new Error("first");
  let error2 = new Error("second");
  let error3 = new Error("third");
  let errors = new DisposableStack();
  errors.defer(() => { throw error1; });
  errors.defer(() => { throw error2; });
  errors.defer(() => { throw error3; });
  try {
    errors.dispose();
  } catch (error) {
    return order.join(",") + ":" +
      moveState + ":" +
      asyncMoveState + ":" +
      stack.disposed + ":" +
      (error instanceof SuppressedError) + ":" +
      (error.error === error1) + ":" +
      (error.suppressed instanceof SuppressedError) + ":" +
      (error.suppressed.error === error2) + ":" +
      (error.suppressed.suppressed === error3);
  }
  return "missing throw";
})()
