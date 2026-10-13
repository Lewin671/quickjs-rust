eval("class B { constructor() { this.x = 4; } }\n              class D extends B { constructor() { eval('super()'); } }\n              [new D().x, new D().x].join(',');")
