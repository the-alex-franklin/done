interface MemoryUsage {
  mallocSize: number;
  mallocLimit: number;
  memoryUsedSize: number;
  mallocCount: number;
  objCount: number;
  objSize: number;
  strCount: number;
  strSize: number;
  jsFuncCount: number;
}

declare function gc(): void;
declare function memoryUsage(): MemoryUsage;

declare function setTimeout(cb: () => void, ms?: number): number;
declare function clearTimeout(id: number): void;
declare function setInterval(cb: () => void, ms?: number): number;
declare function clearInterval(id: number): void;

declare namespace fs {
  function readFileSync(path: string): string;
  function writeFileSync(path: string, data: string): void;
  function appendFileSync(path: string, data: string): void;
  function existsSync(path: string): boolean;
  function unlinkSync(path: string): void;
}
