(function() { let value = 2; let object = { toString: function() { value += 3; return value; } }; return `start:${1 + 1}:${object}:end`; })()
