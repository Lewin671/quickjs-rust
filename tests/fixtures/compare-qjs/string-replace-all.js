"foo foo".replaceAll("foo", "bar") + ":" +
    "abc".replaceAll("", "-") + ":" +
    "aba".replaceAll("a", "[$&:$`:$']") + ":" +
    "a-b-a".replaceAll("a", function(match, position, input) { return match + position + input.length; }) + ":" +
    "a1b2".replaceAll(/(\d)/g, "[$1:$&]") + ":" +
    String.prototype.replaceAll.length
