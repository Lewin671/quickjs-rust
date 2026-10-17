(function() {
  function show(m) { return m === null ? "null" : JSON.stringify(Array.prototype.slice.call(m)) + "@" + m.index; }
  var out = [];
  out.push(show(/a{3,}?$/.exec("aa")));
  out.push(show(/a{2,}?/.exec("aaaa")));
  out.push(show(/a{1,2}?b/.exec("aaab")));
  out.push(show(/a{0}?b/.exec("ab")));
  out.push(show(/^[^x]*?;/.exec("abx;")));
  out.push(show(/"([^]*?)"/.exec('x"ab"cd"')));
  out.push(show(/(\w+?)=\1;/.exec("ab=ab;")));
  out.push(show(/[\s\S]*?(?!a)/.exec("aab")));
  out.push(show(/^(.+?)x$/u.exec("\u{1F600}\u{1F600}x")));
  out.push(show(/^.+?/.exec("\u{1F600}x")));
  out.push(show(/(\p{L}+?)1/u.exec("ab1")));
  out.push(show(/[A-Z]*?c/i.exec("ABC")));
  out.push('"a","bc","",d'.replace(/"([^]*?)"/gm, "<$1>"));
  out.push('"a"\n"b\nc"'.replace(/"([^]*?)"/g, "[$1]"));
  out.push("a1,a22,".replace(/a.*?,/g, "-"));
  out.push("abc;d;".replace(/[^]*?(?=;)/g, "-"));
  var sticky = /([^x]*?);/y, seen = [];
  var s = "ab;cd;ex;";
  var m;
  while ((m = sticky.exec(s))) seen.push(m[1] + ":" + sticky.lastIndex);
  seen.push("end:" + sticky.lastIndex);
  out.push(seen.join(","));
  var g = /<(.*?)>/g, tags = [];
  while ((m = g.exec("<a><bb><>"))) tags.push(m[1] + "@" + m.index + ":" + g.lastIndex);
  out.push(tags.join(","));
  out.push("\u{1F600}a\u{1F600}b".replace(/(.*?)b/gu, "[$1]"));
  out.push("x".repeat(4) + "abc".replace(/(.+)/g, "<$&|$1>") + "ab\ncd".replace(/(.+)/g, "[$1]"));
  return out.join("|").split("\n").join("\\n");
})()
