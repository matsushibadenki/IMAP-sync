import {spawnSync} from 'node:child_process';
import {existsSync} from 'node:fs';
import {resolve,dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
export const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
export const build=join(root,'.engine-build');
export const perl=process.env.IMAPSYNC_BUILD_PERL||(existsSync(join(build,'perl-runtime/bin/perl'))?join(build,'perl-runtime/bin/perl'):'perl');
export function run(cmd,args,options={}) {
 const result=spawnSync(cmd,args,{encoding:'utf8',maxBuffer:16*1024*1024,...options});
 if(result.error||result.status!==0)throw Error(`${cmd} failed: ${result.error?.message||result.stderr||result.stdout}`);
 return result.stdout?.trim()||'';
}
export const perlVersion=run(perl,['-e','print $^V']);
export const localLib=join(build,'modules',perlVersion.replaceAll(/[^a-zA-Z0-9.]/g,''),'lib/perl5');
export function buildEnvironment() {
 const env={...process.env,PERL5LIB:localLib};
 if(process.platform==='darwin'&&!env.OPENSSL_PREFIX&&existsSync(join(build,'openssl/lib/libssl.3.dylib')))env.OPENSSL_PREFIX=join(build,'openssl');
 return env;
}
