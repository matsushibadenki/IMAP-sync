// Build-time only: package the vendored Perl source + interpreter/modules with PAR.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,existsSync,mkdirSync,chmodSync,renameSync,copyFileSync} from 'node:fs';
import {dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {nativeLibraries} from './imapsync-native-libs.mjs';
import {root,build,perl,perlVersion,localLib,buildEnvironment,run} from './imapsync-build-utils.mjs';
process.chdir(root);
Object.assign(process.env,buildEnvironment());
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const source=join(root,'vendor/imapsync/imapsync'), provenance=JSON.parse(readFileSync(join(root,'vendor/imapsync/source.json')));
if(hash(readFileSync(source))!==provenance.sha256)throw Error('Vendored imapsync source checksum mismatch');
const host=run('rustc',['--print','host-tuple']);
const target=process.env.TAURI_ENV_TARGET_TRIPLE||host;
if(target!==host)throw Error(`Build imapsync natively for ${target}; cross-packaging a ${host} Perl runtime is not supported.`);
const extension=process.platform==='win32'?'.exe':'';
const output=join(root,`src-tauri/binaries/imapsync-engine-${host}${extension}`);
const metadata=output+'.json';
const versionModules=['PAR::Packer','Mail::IMAPClient','IO::Socket::SSL','Net::SSLeay','Authen::SASL','Authen::NTLM','Encode::IMAPUTF7','Sys::MemInfo','Mozilla::CA'];
const moduleVersions=JSON.parse(run(perl,['-MJSON::PP','-e','my %v; for my $m(@ARGV){eval "require $m"; die $@ if $@; no strict "refs"; $v{$m}="".($m->VERSION//0);} print JSON::PP->new->canonical->encode({%v});',...versionModules]));
const sslInputs=process.platform==='darwin'&&process.env.OPENSSL_PREFIX?['libssl.3.dylib','libcrypto.3.dylib'].map(name=>readFileSync(join(process.env.OPENSSL_PREFIX,'lib',name))):[];
const fingerprint=hash(Buffer.concat([readFileSync(fileURLToPath(import.meta.url)),readFileSync(source),readFileSync(join(root,'scripts/imapsync-build-utils.mjs')),readFileSync(join(root,'scripts/imapsync-native-libs.mjs')),readFileSync(join(root,'scripts/engine-inventory.pl')),Buffer.from(perlVersion),Buffer.from(JSON.stringify(moduleVersions)),...sslInputs]));
if(!process.argv.includes('--force')&&existsSync(output)&&existsSync(metadata)) {
 const info=JSON.parse(readFileSync(metadata));
 if(info.fingerprint===fingerprint&&info.sha256===hash(readFileSync(output))) {console.log(`Bundled imapsync ready (${host})`);process.exit(0);}
}
if(process.argv.includes('--check'))throw Error('Bundled imapsync is missing or stale. Run npm run engine:prepare.');
const modules=['Authen::NTLM','Authen::SASL::Perl::PLAIN','Authen::SASL::Perl::LOGIN','Authen::SASL::Perl::CRAM_MD5','Authen::SASL::Perl::DIGEST_MD5','IO::Socket::IP','Mozilla::CA'];
run(perl,['-MPAR::Packer','-e','exit 0']);
const ca=run(perl,['-MMozilla::CA','-e','print Mozilla::CA::SSL_ca_file()']);
mkdirSync(dirname(output),{recursive:true});
const temporary=output+'.building'+extension;
const args=['-Mpp','-e','pp->go','--','-x','--xargs=--version'];
for(const module of modules)args.push('-M',module);
const libraries=nativeLibraries(localLib);
for(const library of libraries)args.push('-l',library);
args.push('-a',`${ca};lib/Mozilla/CA/cacert.pem`,'-o',temporary,source);
console.log(`Packing imapsync ${provenance.version} for ${host}...`);
run(perl,args,{stdio:'inherit'});
// Fail before publishing a bad build. No network or real accounts are used.
const cleanEnv={...process.env,PATH:process.platform==='win32'?process.env.SystemRoot+'\\System32':'/usr/bin:/bin:/usr/sbin:/sbin'};
for(const key of Object.keys(cleanEnv))if(key.startsWith('PERL')||key.startsWith('PAR_'))delete cleanEnv[key];
const version=run(temporary,['--version'],{env:cleanEnv});
if(version.split(/\s/).at(-1)!==provenance.version)throw Error(`Unexpected bundled imapsync version: ${version}`);
const inventory=run(perl,[join(root,'scripts/engine-inventory.pl'),temporary]);
renameSync(temporary,output);if(process.platform!=='win32')chmodSync(output,0o755);
const info={version:provenance.version,target:host,sourceSha256:provenance.sha256,sha256:hash(readFileSync(output)),fingerprint,perl:run(perl,['-e','print $^V']),modules:moduleVersions,libraries:libraries.map(path=>({name:path.split(/[\\/]/).at(-1),sha256:hash(readFileSync(path))})),builtAt:new Date().toISOString()};
writeFileSync(metadata,JSON.stringify(info,null,2)+'\n');
mkdirSync(join(root,'src-tauri/engine-notices'),{recursive:true});
if(process.env.OPENSSL_PREFIX) {
 const dest=join(root,'src-tauri/engine-notices/licenses/OpenSSL');mkdirSync(dest,{recursive:true});
 for(const name of ['LICENSE.txt','AUTHORS.md','README.md'])if(existsSync(join(process.env.OPENSSL_PREFIX,name))){copyFileSync(join(process.env.OPENSSL_PREFIX,name),join(dest,name));chmodSync(join(dest,name),0o644);}
}
writeFileSync(join(root,'src-tauri/engine-notices/inventory.json'),inventory+'\n');
writeFileSync(join(root,'src-tauri/engine-notices/build.json'),JSON.stringify(info,null,2)+'\n');
console.log(`Created ${output}`);
