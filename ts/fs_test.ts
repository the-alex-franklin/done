const path = "/tmp/done_fs_test.txt";

// write
fs.writeFileSync(path, "hello from done\n");
console.log("exists after write:", fs.existsSync(path));

// read back
const content = fs.readFileSync(path);
console.log("content:", content.trim());

// append
fs.appendFileSync(path, "second line\n");
const content2 = fs.readFileSync(path);
console.log("after append:", content2.trim());

// delete
fs.unlinkSync(path);
console.log("exists after unlink:", fs.existsSync(path));

// error on missing file
try {
  fs.readFileSync("/tmp/does_not_exist_done.txt");
} catch (e: any) {
  console.log("caught expected error:", e.message.slice(0, 40));
}
