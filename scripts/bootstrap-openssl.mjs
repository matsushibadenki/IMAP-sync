// macOS build-only TLS toolchain. No Homebrew installation is used or modified.
import {mkdirSync,existsSync,writeFileSync,readFileSync,copyFileSync,chmodSync} from 'node:fs';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {arch} from 'node:os';
import {root,build,perl,run} from './imapsync-build-utils.mjs';
if(process.platform!=='darwin')throw Error('This source bootstrap is for macOS. Other platforms use their native OpenSSL development toolchain.');
const version='3.5.9',sha256='603f5602e2eef00d77fbd429d34dcd5822bb301757a1bc9cdb24c670f1eb859a';
const prefix=join(build,'openssl');
if(existsSync(join(prefix,'lib/libssl.3.dylib'))){console.log('Private OpenSSL is ready.');process.exit(0);}
mkdirSync(build,{recursive:true});
const archive=join(build,`openssl-${version}.tar.gz`);
if(!existsSync(archive)) {
 const response=await fetch(`https://github.com/openssl/openssl/releases/download/openssl-${version}/openssl-${version}.tar.gz`);
 if(!response.ok)throw Error(`OpenSSL download failed: ${response.status}`);
 writeFileSync(archive,Buffer.from(await response.arrayBuffer()));
}
if(createHash('sha256').update(readFileSync(archive)).digest('hex')!==sha256)throw Error('OpenSSL source checksum mismatch');
const sources=join(build,'openssl-source');mkdirSync(sources,{recursive:true});run('tar',['-xzf',archive,'-C',sources]);
const cwd=join(sources,`openssl-${version}`),minimum=process.env.MACOSX_DEPLOYMENT_TARGET||'11.0';
const options={cwd,stdio:'inherit',env:{...process.env,MACOSX_DEPLOYMENT_TARGET:minimum}};
run(perl,['Configure',arch()==='arm64'?'darwin64-arm64-cc':'darwin64-x86_64-cc','shared',`--prefix=${prefix}`,`--openssldir=${prefix}/ssl`,'--libdir=lib',`-mmacosx-version-min=${minimum}`],options);
run('make',['-j4'],options);run('make',['install_sw'],options);
copyFileSync(join(cwd,'LICENSE.txt'),join(root,'vendor/licenses/OpenSSL-Apache-2.0.txt'));
chmodSync(join(root,'vendor/licenses/OpenSSL-Apache-2.0.txt'),0o644);
console.log('Private OpenSSL installed. Run npm run engine:deps.');
