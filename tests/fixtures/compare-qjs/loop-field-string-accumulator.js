eval("function f(n){var o={pear:2,extra:3},sum=0,guard=0;for(var i=0;i<n&&guard<23;i++,guard++){if(i===3)sum='q';sum+=o.pear+o.extra;}return sum;}[f(0),f(1),f(3),f(7),f(2)].join('|');\n")
