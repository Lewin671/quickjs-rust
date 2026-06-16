// Derived from: test/built-ins/RegExp/property-escapes/generated/General_Category_-_Surrogate.js
var high = "\uD83D";
var low = "\uDE00";
var pair = high + low;

if (!/^\p{Surrogate}$/u.test(high)) { throw; }
if (!/^\p{Surrogate}$/u.test(low)) { throw; }
if (/^\p{Surrogate}$/u.test(pair)) { throw; }

if (/^\P{Surrogate}$/u.test(high)) { throw; }
if (/^\P{Surrogate}$/u.test(low)) { throw; }
if (!/^\P{Surrogate}$/u.test(pair)) { throw; }
