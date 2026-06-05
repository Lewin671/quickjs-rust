// Derived from: test/annexB/built-ins/String/prototype/anchor/B.2.3.2.js
// Derived from: test/annexB/built-ins/String/prototype/bold/B.2.3.5.js
// Derived from: test/annexB/built-ins/String/prototype/fontcolor/B.2.3.7.js
// Derived from: test/annexB/built-ins/String/prototype/link/B.2.3.10.js
if ('x'.anchor('a"b') !== '<a name="a&quot;b">x</a>') {
  throw 'expected anchor to escape attribute quotes';
}
if ('x'.big() !== '<big>x</big>') {
  throw 'expected big wrapper';
}
if ('x'.blink() !== '<blink>x</blink>') {
  throw 'expected blink wrapper';
}
if ('x'.bold() !== '<b>x</b>') {
  throw 'expected bold wrapper';
}
if ('x'.fixed() !== '<tt>x</tt>') {
  throw 'expected fixed wrapper';
}
if ('x'.fontcolor('red') !== '<font color="red">x</font>') {
  throw 'expected fontcolor wrapper';
}
if ('x'.fontsize(3) !== '<font size="3">x</font>') {
  throw 'expected fontsize wrapper';
}
if ('x'.italics() !== '<i>x</i>') {
  throw 'expected italics wrapper';
}
if ('x'.link('https://e.test') !== '<a href="https://e.test">x</a>') {
  throw 'expected link wrapper';
}
if ('x'.small() !== '<small>x</small>') {
  throw 'expected small wrapper';
}
if ('x'.strike() !== '<strike>x</strike>') {
  throw 'expected strike wrapper';
}
if ('x'.sub() !== '<sub>x</sub>') {
  throw 'expected sub wrapper';
}
if ('x'.sup() !== '<sup>x</sup>') {
  throw 'expected sup wrapper';
}
if (String.prototype.bold.length !== 0 || String.prototype.link.length !== 1) {
  throw 'expected HTML method lengths';
}
