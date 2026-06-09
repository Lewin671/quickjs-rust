"foo foo".replaceAll("foo", "bar") + ":" +
    "abc".replaceAll("", "-") + ":" +
    "aba".replaceAll("a", "[$&:$`:$']") + ":" +
    "a-b-a".replaceAll("a", function(match, position, input) { return match + position + input.length; }) + ":" +
    "a1b2".replaceAll(/(\d)/g, "[$1:$&]") + ":" +
    RegExp.prototype[Symbol.replace].call(/a(.)/g, "a1 a2", "[$1:$&]") + ":" +
    String.prototype.replaceAll.length
