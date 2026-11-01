// Function writes implicit globals each iteration (string-validate-input shape):
// candidate rejects it from the wide tier and runs reference events by name.
function mk(n) { var t = ""; for (var i = 0; i < n; i++) t += "a"; return t; }
function doTest() {
    for (var k = 0; k < 100000; k++) {
        username = mk(3);
        email = username + "@x";
    }
}
doTest();
if (email !== "aaa@x") throw new Error("bad");
