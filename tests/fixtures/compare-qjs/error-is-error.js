(function () {
  return Error.isError(new Error("boom")) + ":" +
    Error.isError(Error("boom")) + ":" +
    Error.isError(new TypeError("boom")) + ":" +
    Error.isError(new AggregateError([], "boom")) + ":" +
    Error.isError({}) + ":" +
    Error.isError(Error) + ":" +
    Error.isError() + ":" +
    Error.isError("boom");
})()
