class Resource {
  name: string;
  constructor(name: string) {
    this.name = name;
    console.log(`${name}: acquired`);
  }

  // @ts-ignore
  [Symbol.dispose]() {
    console.log(`${this.name}: disposed`);
  }
}

function doWork() {
  using r = new Resource("db-connection");
  console.log("doing work with", r.name);
}

doWork();
console.log("after doWork");
