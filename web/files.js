// Files the player keeps: the house Blob download and file-input load
// (gear-master-2d web/app.js:4659-4661, :5290).

export function download(bytes, name, type = 'application/octet-stream') {
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

// Resolves with the chosen file's bytes, or null if nothing was chosen.
export function pick(accept) {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.id = 'file-pick';
    input.hidden = true;
    input.addEventListener('change', async () => {
      const f = input.files && input.files[0];
      input.remove();
      resolve(f ? { name: f.name, type: f.type, bytes: new Uint8Array(await f.arrayBuffer()) } : null);
    });
    document.body.appendChild(input);
    input.click();
  });
}
