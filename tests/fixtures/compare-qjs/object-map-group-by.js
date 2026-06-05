(function () {
    let objectGroups = Object.groupBy([1, 2, 3, 4], function (value) {
        return value % 2 ? "odd" : "even";
    });
    let seen = "";
    let mapGroups = Map.groupBy(["a", "b"], function (value, index) {
        seen = seen + value + index;
        return index;
    });
    let key = {};
    let identityGroups = Map.groupBy(["x", "y"], function (value) {
        return value === "x" ? key : {};
    });
    return Object.getPrototypeOf(objectGroups) + ":" +
        objectGroups.odd.join("|") + ":" +
        objectGroups.even.join("|") + ":" +
        mapGroups.get(0)[0] + ":" +
        mapGroups.get(1)[0] + ":" +
        seen + ":" +
        identityGroups.get(key)[0] + ":" +
        identityGroups.size + ":" +
        Object.groupBy.length + ":" +
        Map.groupBy.length;
})()
