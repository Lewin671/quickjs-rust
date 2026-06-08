(function() { function tag(strings, value) { return strings[0] + ":" + strings.raw[0] + ":" + value; } return tag`A${2 + 3}`; })()
