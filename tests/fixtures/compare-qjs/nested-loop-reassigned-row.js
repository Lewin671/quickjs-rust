(function () {
  /* A nested loop whose inner receiver is reassigned by the outer loop: the
     inner element reads and writes must follow the current row, not the array
     the loop region first saw. */
  var rows = [[10, 5, 7], [20, 5, 7], [30, 5, 7]];
  function sumRows() {
    var sum = 0;
    for (var i = 0; i < rows.length; ++i) {
      var row = rows[i];
      for (var j = 0; j < row.length; ++j) sum += row[j];
    }
    return sum;
  }
  function scaleRows(factor) {
    for (var i = 0; i < rows.length; ++i) {
      var row = rows[i];
      for (var j = 0; j < row.length; ++j) row[j] = row[j] * factor;
    }
  }
  function CreateP(x, y, z) { this.V = [x, y, z, 1]; }
  var points = [];
  for (var k = 0; k < 9; k++) points[k] = new CreateP(k * 1.5, k * 2.25, k * 3.125);
  function sumPoints() {
    var sum = 0;
    for (var i = 0; i < points.length; ++i) {
      var vector = points[i].V;
      for (var j = 0; j < vector.length; ++j) sum += vector[j];
    }
    return sum;
  }
  var before = sumRows();
  scaleRows(2);
  return before + ":" + sumRows() + ":" + rows.join(";") + ":" + sumPoints();
})()
