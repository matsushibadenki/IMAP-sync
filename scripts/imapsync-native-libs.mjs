import {readdirSync,existsSync,mkdirSync,copyFileSync,chmodSync} from 'node:fs';
import {join,dirname,basename,resolve} from 'node:path';
import {run,build,perl} from './imapsync-build-utils.mjs';
function copy(source,dest) {if(existsSync(dest))chmodSync(dest,0o644);copyFileSync(source,dest);chmodSync(dest,0o755);}
function files(dir) {return existsSync(dir)?readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?files(join(dir,e.name)):[join(dir,e.name)]):[];}
export function nativeLibraries(localLib) {
 const libraries=new Map(), staging=join(build,'native-libs');mkdirSync(staging,{recursive:true});
 if(process.platform==='darwin') {
  const pending=files(localLib).filter(p=>p.endsWith('.bundle'));
  const dependencies=file=>run('otool',['-L',file]).split('\n').slice(1).map(line=>line.trim().split(' (')[0]);
  const add=path=>{
   const name=basename(path),dest=join(staging,name);
   if(libraries.has(name))return;
   if(!existsSync(path))throw Error(`Native library not found: ${path}`);
   copy(path,dest);libraries.set(name,dest);
   for(const dep of dependencies(path))if(dep.startsWith('/')&&!dep.startsWith('/usr/lib/')&&!dep.startsWith('/System/Library/'))add(dep);
  };
  for(const file of pending)for(const dep of dependencies(file))if(dep.startsWith('/')&&!dep.startsWith('/usr/lib/')&&!dep.startsWith('/System/Library/'))add(dep);
  // Keep generated module trees runnable for PAR's dynamic dependency scan, and
  // use flat @loader_path links matching PAR's extraction directory at runtime.
  for(const file of [...pending,...libraries.values()]) {
   let changed=false;
   if(file.endsWith('.dylib')) {run('install_name_tool',['-id',`@loader_path/${basename(file)}`,file]);changed=true;}
   for(const dep of dependencies(file)) {
    const name=basename(dep);
    if(libraries.has(name)&&dep!==`@loader_path/${name}`) {run('install_name_tool',['-change',dep,`@loader_path/${name}`,file]);changed=true;}
   }
   if(changed)run('codesign',['--force','--sign','-',file]);
  }
  for(const file of pending)for(const [name,lib] of libraries)if(dirname(file)!==staging)copy(lib,join(dirname(file),name));
  // On a repeated build, patched XS libraries already use relative references.
  for(const file of files(staging))if(file.endsWith('.dylib'))libraries.set(basename(file),file);
 } else if(process.platform==='win32') {
  const root=resolve(dirname(perl),'../..');
  const dirs=[dirname(perl),join(root,'c/bin'),...String(process.env.PATH||'').split(';')];
  for(const dir of dirs)if(existsSync(dir))for(const file of readdirSync(dir)) {
   if(/^(lib(?:ssl|crypto|gcc|winpthread|stdc\+\+|ssp|iconv|intl)|zlib).*\.dll$/i.test(file))libraries.set(file,join(dir,file));
  }
 }
 return [...libraries.values()];
}
