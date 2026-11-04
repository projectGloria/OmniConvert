import { Fd, Inode, OpenFile, wasi } from "@bjorn3/browser_wasi_shim";

/** Worker-only API, absent from the DOM lib this project type-checks against. */
declare class FileReaderSync {
  readAsArrayBuffer(blob: Blob): ArrayBuffer;
}

/**
 * Read-only WASI file backed by a DOM Blob, read lazily with FileReaderSync (workers
 * only). The shim's own File needs the bytes in a Uint8Array up front — fine for a
 * 2 MB PNG, wasteful for a 900 MB archive that we'd copy into memory just so the engine
 * can stream it again. Here each read() pulls exactly the requested slice.
 */
export class BlobFile extends Inode {
  readonly reader = new FileReaderSync();
  constructor(readonly blob: Blob) {
    super();
  }

  path_open(oflags: number, rights: bigint) {
    const writes = (rights & BigInt(wasi.RIGHTS_FD_WRITE)) !== 0n;
    if (writes || (oflags & wasi.OFLAGS_TRUNC) !== 0) return { ret: wasi.ERRNO_PERM, fd_obj: null };
    return { ret: wasi.ERRNO_SUCCESS, fd_obj: new OpenBlobFile(this) };
  }

  stat() {
    return new wasi.Filestat(this.ino, wasi.FILETYPE_REGULAR_FILE, BigInt(this.blob.size));
  }
}

class OpenBlobFile extends Fd {
  private pos = 0n;
  constructor(private readonly file: BlobFile) {
    super();
  }

  private slice(at: number, size: number): Uint8Array {
    const part = this.file.blob.slice(at, at + size);
    return new Uint8Array(this.file.reader.readAsArrayBuffer(part));
  }

  override fd_read(size: number) {
    const data = this.slice(Number(this.pos), size);
    this.pos += BigInt(data.byteLength);
    return { ret: wasi.ERRNO_SUCCESS, data };
  }

  override fd_pread(size: number, offset: bigint) {
    return { ret: wasi.ERRNO_SUCCESS, data: this.slice(Number(offset), size) };
  }

  override fd_seek(offset: bigint, whence: number) {
    const base = whence === wasi.WHENCE_CUR ? this.pos : whence === wasi.WHENCE_END ? BigInt(this.file.blob.size) : 0n;
    const next = base + offset;
    if (next < 0n) return { ret: wasi.ERRNO_INVAL, offset: 0n };
    this.pos = next;
    return { ret: wasi.ERRNO_SUCCESS, offset: next };
  }

  override fd_tell() {
    return { ret: wasi.ERRNO_SUCCESS, offset: this.pos };
  }

  override fd_fdstat_get() {
    return { ret: wasi.ERRNO_SUCCESS, fdstat: new wasi.Fdstat(wasi.FILETYPE_REGULAR_FILE, 0) };
  }

  override fd_filestat_get() {
    return { ret: wasi.ERRNO_SUCCESS, filestat: this.file.stat() };
  }
}

/**
 * The shim's in-memory OpenFile.fd_write reallocates the *whole* file on every write
 * that extends it — O(n²) for an output written through an 8 KB BufWriter (a 100 MB
 * archive would copy ~600 GB). Patch it to grow capacity geometrically: `file.data`
 * stays an exact-length *view* (the shim reads size from data.byteLength) over a larger
 * buffer. Pinned shim version (package-lock); this runs only inside the engine worker.
 */
export function patchGrowableWrites(): void {
  OpenFile.prototype.fd_write = function (this: OpenFile, data: Uint8Array) {
    if (this.file.readonly) return { ret: wasi.ERRNO_BADF, nwritten: 0 };
    const pos = Number(this.file_pos);
    const end = pos + data.byteLength;
    let view = this.file.data;
    if (end > view.byteLength) {
      const capacity = view.buffer.byteLength - view.byteOffset;
      if (end <= capacity) {
        view = new Uint8Array(view.buffer, view.byteOffset, end);
      } else {
        const grown = new Uint8Array(Math.max(end, capacity * 2, 1 << 16));
        grown.set(view);
        view = grown.subarray(0, end);
      }
      this.file.data = view;
    }
    view.set(data, pos);
    this.file_pos = BigInt(end);
    return { ret: wasi.ERRNO_SUCCESS, nwritten: data.byteLength };
  };
}
