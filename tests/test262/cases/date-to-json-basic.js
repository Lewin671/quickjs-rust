// Derived from: test/built-ins/Date/prototype/toJSON/invoke-result.js
if (Date.prototype.toJSON.length !== 1) { throw; }
if (new Date("1970-01-02T03:04:05.006Z").toJSON() !== "1970-01-02T03:04:05.006Z") { throw; }
if (new Date(NaN).toJSON() !== null) { throw; }
if (JSON.stringify(new Date("1970-01-02T03:04:05.006Z")) !== '"1970-01-02T03:04:05.006Z"') { throw; }
if (JSON.stringify({ when: new Date("1970-01-02T03:04:05.006Z") }) !== '{"when":"1970-01-02T03:04:05.006Z"}') { throw; }
var custom = new Date("1970-01-02T03:04:05.006Z");
custom.toISOString = function () { return "custom"; };
if (custom.toJSON() !== "custom") { throw; }
if (JSON.stringify(custom) !== '"custom"') { throw; }
if (JSON.stringify(new Date(NaN)) !== "null") { throw; }
