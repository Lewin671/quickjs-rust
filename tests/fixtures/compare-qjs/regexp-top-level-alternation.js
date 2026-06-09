(function() {
  var first = /1|12/.exec("123");
  var second = /2|12/.exec(1.012);
  var third = /AL|se/.exec(new Boolean(false));
  var fourth = /ll|l/.exec(null);
  var fifth = /nd|ne/.exec(undefined);
  return [
    first[0] + "@" + first.index,
    second[0] + "@" + second.index,
    third[0] + "@" + third.index,
    fourth[0] + "@" + fourth.index,
    fifth[0] + "@" + fifth.index
  ].join("|");
})()
