(function () {
  try {
    new RegExp("^[z-a]$");
    return "missing";
  } catch (error) {
    return error instanceof SyntaxError;
  }
})()
