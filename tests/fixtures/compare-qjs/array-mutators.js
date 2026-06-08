(function () {
  let xs = [1];
  let ys = xs;
  let pushed = xs.push(2, 3);
  let popped = ys.pop();
  xs[3] = 4;
  let grown = xs.length;
  xs.length = 2;

  let caughtEmpty = false;
  let caughtText = false;
  try {
    Array.prototype.pop.call("");
  } catch (error) {
    caughtEmpty = error instanceof TypeError;
  }
  try {
    Array.prototype.pop.call("abc");
  } catch (error) {
    caughtText = error instanceof TypeError;
  }

  return pushed
    + ":" + popped
    + ":" + grown
    + ":" + xs.length
    + ":" + xs.join()
    + ":" + (xs === ys)
    + ":" + ([] === [])
    + ":" + caughtEmpty
    + ":" + caughtText;
})()
