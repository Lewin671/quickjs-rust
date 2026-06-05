// Derived from: test/built-ins/String/prototype/replaceAll/searchValue-empty-string.js
// Derived from: test/built-ins/String/prototype/replaceAll/replaceValue-call-each-match-position.js
var empty = 'aab c  \nx'.replaceAll('', '_');
if (empty !== '_a_a_b_ _c_ _ _\n_x_') {
  throw 'expected replaceAll to replace every empty-string position';
}

var calls = '';
var result = 'ab c ab cdab cab c'.replaceAll('ab c', function(match, position, string) {
  calls += match + '@' + position + '/' + string.length + ';';
  return 'z';
});

if (result !== 'z zdzz') {
  throw 'expected replaceAll functional replacement result';
}
if (calls !== 'ab c@0/18;ab c@5/18;ab c@10/18;ab c@14/18;') {
  throw 'expected replaceAll to call replacer at each match position';
}
