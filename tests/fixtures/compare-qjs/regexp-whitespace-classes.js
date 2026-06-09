(function () {
  return [
    /\s/.test("\u00a0"),
    /\s/.test("\u0085"),
    /\S/.test("\u0085"),
    /\S/.test("\u180e"),
    /[\s]/.test("\u202f"),
    /[\S]/.test("\u0085"),
    "x\u0085y".replace(/\S+/g, "z")
  ].join(":");
})()
