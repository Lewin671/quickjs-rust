String.prototype.isWellFormed.length + ":" +
String.prototype.toWellFormed.length + ":" +
Object.getOwnPropertyDescriptor(String.prototype, "isWellFormed").enumerable + ":" +
Object.getOwnPropertyDescriptor(String.prototype, "toWellFormed").enumerable + ":" +
"abc".isWellFormed() + ":" +
"\uD83D\uDCA9".isWellFormed() + ":" +
"\uD83D".isWellFormed() + ":" +
"\uDCA9".isWellFormed() + ":" +
"\uD83D".toWellFormed().charCodeAt(0) + ":" +
"\uDCA9A".toWellFormed().charCodeAt(0) + ":" +
("\uD83D\uDCA9".toWellFormed() === "\uD83D\uDCA9")
