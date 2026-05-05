const message: string = "Hello from Done!";
const nums: number[] = [1, 2, 3];

console.log(message);
console.log("sum:", nums.reduce((a, b) => a + b, 0).toString());

console.log({ a: 1, b: 2 }.toString());
