// setTimeout fires in order
setTimeout(() => console.log("timeout 100ms"), 100);
setTimeout(() => console.log("timeout 50ms"), 50);
setTimeout(() => console.log("timeout 0ms"), 0);

// setInterval fires 3 times then clears itself
let count = 0;
const id = setInterval(() => {
  count++;
  console.log("interval tick", count);
  if (count === 3) clearInterval(id);
}, 30);

console.log("sync: before event loop");
