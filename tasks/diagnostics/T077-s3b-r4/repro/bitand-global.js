// Implicit (configurable) global written in a top-level counted loop.
g = 4294967296;
for (var i = 0; i < 600000; i++)
    g = g & i;
if (g !== 0) throw new Error("bad " + g);
