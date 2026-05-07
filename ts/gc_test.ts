// create a reference cycle: a → b → a
let a: any = {};
let b: any = { ref: a };
a.ref = b;

// drop both — ref count can't hit zero because they still point at each other
a = null;
b = null;

console.log("with cycle (not yet collected):", memoryUsage().objCount, "objects");
gc();
console.log("after gc (cycle collected):", memoryUsage().objCount, "objects");
