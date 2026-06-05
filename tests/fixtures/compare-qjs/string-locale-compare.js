(function () {
    function sign(value) {
        return value < 0 ? -1 : value > 0 ? 1 : 0;
    }

    return sign("abc".localeCompare("abc")) + ":" +
        sign("abc".localeCompare("abd")) + ":" +
        sign("abd".localeCompare("abc")) + ":" +
        sign(String.prototype.localeCompare.call(123, "123")) + ":" +
        sign("undefined".localeCompare()) + ":" +
        String.prototype.localeCompare.length;
})()
