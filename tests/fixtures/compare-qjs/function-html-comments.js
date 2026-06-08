(function () {
  var results = [];
  results.push(Function("<!--")() === undefined);
  results.push(Function("-->")() === undefined);
  results.push(Function("<!--", "")() === undefined);
  results.push(Function("\n-->", "")() === undefined);
  try {
    Function("-->", "");
    results.push("missing");
  } catch (error) {
    results.push(error instanceof SyntaxError);
  }
  return results.join(":");
})()
