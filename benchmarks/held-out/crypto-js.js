// Held-out driver: crypto-js (MIT) hashes and encrypts a generated payload.
// Deterministic. Prints one line.
var N = 6000;

var payload = "";
for (var i = 0; i < 64; i++) {
  payload += String.fromCharCode(97 + (i % 26));
}

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
for (var iteration = 0; iteration < N; iteration++) {
  var text = payload + iteration;
  var sha = CryptoJS.SHA256(text).toString();
  var md5 = CryptoJS.MD5(text).toString();
  var hmac = CryptoJS.HmacSHA1(text, "key" + (iteration % 8)).toString();
  mix(sha.charCodeAt(iteration % 64));
  mix(md5.charCodeAt(iteration % 32));
  mix(hmac.charCodeAt(iteration % 40));
}
print("CHECKSUM " + (hash >>> 0).toString(16));
