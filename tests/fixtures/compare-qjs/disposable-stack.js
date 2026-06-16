(function () {
  let order = [];
  let stack = new DisposableStack();
  let resource = { [Symbol.dispose]() { order.push("use"); } };
  stack.use(resource);
  stack.adopt("value", value => order.push("adopt:" + value));
  stack.defer(() => order.push("defer"));
  stack.dispose();
  stack.dispose();

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
      stack.disposed + ":" +
      (error instanceof SuppressedError) + ":" +
      (error.error === error1) + ":" +
      (error.suppressed instanceof SuppressedError) + ":" +
      (error.suppressed.error === error2) + ":" +
      (error.suppressed.suppressed === error3);
  }
  return "missing throw";
})()
