// Developer/CI dependencies only. End users never run this installer.
import {spawnSync} from 'node:child_process';
import {mkdirSync,writeFileSync,readdirSync,copyFileSync,existsSync,chmodSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {root,build,perl,localLib,buildEnvironment} from './imapsync-build-utils.mjs';
mkdirSync(build,{recursive:true});
const url='https://raw.githubusercontent.com/miyagawa/cpanminus/1.7049/cpanm';
const response=await fetch(url);if(!response.ok)throw Error(`Cannot download cpanminus: ${response.status}`);
const cpanm=join(build,'cpanm');writeFileSync(cpanm,await response.text());
const modules=['PAR::Packer@1.064','Digest::HMAC_SHA1','Encode::IMAPUTF7','File::Copy::Recursive','IO::Socket::SSL','IO::Tee','Mail::IMAPClient','Term::ReadKey','Unicode::String','Readonly','Sys::MemInfo','Regexp::Common','File::Tail','Authen::SASL','Authen::NTLM','Mozilla::CA'];
const result=spawnSync(perl,[cpanm,'--no-lwp','--notest','--local-lib-contained',resolve(localLib,'../..'),'--mirror','https://cpan.metacpan.org','--mirror-only',...modules],{stdio:'inherit',env:{...buildEnvironment(),PERL_CPANM_HOME:join(build,'cpanm-home')}});
if(result.error||result.status!==0)throw Error('Perl dependency installation failed. Check .engine-build/cpanm-home/build.log.');
const notices=join(root,'src-tauri/engine-notices/licenses');mkdirSync(notices,{recursive:true});
const work=join(build,'cpanm-home/work');
if(existsSync(work))for(const session of readdirSync(work)) {
 const sessionDir=join(work,session);
 for(const dist of readdirSync(sessionDir,{withFileTypes:true}).filter(e=>e.isDirectory())) {
  const distribution=join(sessionDir,dist.name);
  for(const file of readdirSync(distribution,{withFileTypes:true})) {
   if(file.isFile()&&/^(?:LICENSE|LICENCE|COPYING|COPYRIGHT|ARTISTIC|README)(?:[.\d_-].*)?$/i.test(file.name)) {
    const dest=join(notices,dist.name);mkdirSync(dest,{recursive:true});if(existsSync(join(dest,file.name)))chmodSync(join(dest,file.name),0o644);copyFileSync(join(distribution,file.name),join(dest,file.name));chmodSync(join(dest,file.name),0o644);
   }
  }
 }
}
console.log('Dependencies installed. Run npm run engine:prepare.');
