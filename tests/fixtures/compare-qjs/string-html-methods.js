(function () {
    return ("x".anchor('a"b') === '<a name="a&quot;b">x</a>') + ":" +
        ("x".big() === "<big>x</big>") + ":" +
        ("x".blink() === "<blink>x</blink>") + ":" +
        ("x".bold() === "<b>x</b>") + ":" +
        ("x".fixed() === "<tt>x</tt>") + ":" +
        ("x".fontcolor("red") === '<font color="red">x</font>') + ":" +
        ("x".fontsize(3) === '<font size="3">x</font>') + ":" +
        ("x".italics() === "<i>x</i>") + ":" +
        ("x".link("https://e.test/?q=1") === '<a href="https://e.test/?q=1">x</a>') + ":" +
        ("x".small() === "<small>x</small>") + ":" +
        ("x".strike() === "<strike>x</strike>") + ":" +
        ("x".sub() === "<sub>x</sub>") + ":" +
        ("x".sup() === "<sup>x</sup>") + ":" +
        String.prototype.bold.length + ":" +
        String.prototype.link.length + ":" +
        (String.prototype.bold.call(7) === "<b>7</b>");
})()
