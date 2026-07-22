// Derived from: test/staging/sm/String/unicode-braced.js

function sameValue(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(message + ": expected " + expected + ", got " + actual);
  }
}

sameValue("\u{D800}", String.fromCodePoint(0xD800), "high surrogate escape");
sameValue("\u{DBFF}", String.fromCodePoint(0xDBFF), "last high surrogate escape");
sameValue("\u{DC00}", String.fromCodePoint(0xDC00), "low surrogate escape");
sameValue("\u{DFFF}", String.fromCodePoint(0xDFFF), "last low surrogate escape");
sameValue("\u{F07FF}", String.fromCodePoint(0xF07FF), "last sentinel-range scalar escape");
sameValue("\u{F07FF}".length, 2, "last sentinel-range scalar UTF-16 length");

var direct = "󰀀";
var escaped = "\u{F0000}";
var fromCodePoint = String.fromCodePoint(0xF0000);
var fromCodeUnits = String.fromCharCode(0xDB80, 0xDC00);

sameValue(direct.length, 2, "direct scalar UTF-16 length");
sameValue(direct.codePointAt(0), 0xF0000, "direct scalar code point");
sameValue(direct.charCodeAt(0), 0xDB80, "direct scalar high code unit");
sameValue(direct.charCodeAt(1), 0xDC00, "direct scalar low code unit");
sameValue(direct, escaped, "direct and escaped scalar equality");
sameValue(direct, fromCodePoint, "literal and fromCodePoint equality");
sameValue(direct, fromCodeUnits, "literal and code-unit equality");
sameValue(direct.slice(0, 1), String.fromCharCode(0xDB80), "slice high surrogate");
sameValue(direct.substring(1, 2), String.fromCharCode(0xDC00), "substring low surrogate");
sameValue(direct.substr(0, 1), String.fromCharCode(0xDB80), "substr high surrogate");

sameValue(`󰀀`.length, 2, "template direct scalar length");
sameValue(`\u{F0000}`, fromCodePoint, "template escaped scalar equality");
sameValue(Array.from(direct)[0], direct, "string iterator scalar equality");
sameValue(Array.from(direct)[0].length, 2, "string iterator UTF-16 length");
sameValue(direct.toLowerCase(), direct, "lowercase preserves private-use scalar");
sameValue(direct.toUpperCase(), direct, "uppercase preserves private-use scalar");
sameValue(decodeURIComponent("%F3%B0%80%80"), direct, "URI-decoded scalar equality");
sameValue(decodeURIComponent("%F3%B0%80%80").length, 2, "URI-decoded UTF-16 length");
sameValue(JSON.parse(JSON.stringify(direct)), direct, "JSON round trip");
sameValue(JSON.parse('"\\udb80\\udc00"'), direct, "JSON surrogate-pair escape");
sameValue(JSON.parse('"\\ud800"').charCodeAt(0), 0xD800, "JSON lone surrogate escape");
sameValue(/./u.exec(direct)[0], direct, "unicode regexp dot match");
sameValue(/./u.exec(direct)[0].length, 2, "unicode regexp match UTF-16 length");
sameValue(/./.exec(direct)[0].charCodeAt(0), 0xDB80, "non-unicode regexp code unit");
sameValue(/^(.)$/u.exec(direct)[1], direct, "unicode regexp capture");
sameValue(direct.match(/./gu).length, 1, "unicode regexp global match count");
sameValue(direct.match(/./g).length, 2, "non-unicode regexp global match count");
sameValue(new RegExp(direct, "u").test(direct), true, "regexp constructor scalar");
sameValue(encodeURIComponent(direct), "%F3%B0%80%80", "URI encoding");
sameValue(String.raw({raw: [direct]}), direct, "String.raw propagation");
sameValue(direct.concat(direct).length, 4, "concat UTF-16 length");
sameValue(direct.repeat(2).length, 4, "repeat UTF-16 length");
sameValue("".padEnd(2, direct), direct, "padding propagation");
sameValue(direct.replace(/(.)/u, "$1"), direct, "capture replacement");
sameValue(direct.replaceAll(direct, direct), direct, "replaceAll propagation");
sameValue(direct.split("").length, 2, "split code-unit count");
sameValue(direct.split("")[0].charCodeAt(0), 0xDB80, "split high surrogate");
sameValue(direct.split("")[1].charCodeAt(0), 0xDC00, "split low surrogate");
sameValue(eval("'" + direct + "'"), direct, "eval scalar source");
sameValue(eval("'" + String.fromCharCode(0xD800) + "'").charCodeAt(0), 0xD800, "eval lone surrogate source");
sameValue(Function("return '" + direct + "';")(), direct, "dynamic function scalar source");
