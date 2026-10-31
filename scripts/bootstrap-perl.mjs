// Optional macOS developer bootstrap, isolated from the system Perl installation.
import {mkdirSync,existsSync,writeFileSync,readFileSync,copyFileSync,chmodSync} from 'node:fs';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {root,build,run} from './imapsync-build-utils.mjs';
if(process.platform==='win32')throw Error('Use Strawberry Perl 5.40 on Windows.');
const version='5.40.3',sha256='4c155b4e6160682b38919b55ac319081b898db11857cf18a7d9ffed2648ccaff';
const prefix=join(build,'perl-runtime');
if(existsSync(join(prefix,'bin/perl'))){console.log('Private build Perl is ready.');process.exit(0);}
mkdirSync(build,{recursive:true});
const archive=join(build,`perl-${version}.tar.gz`);
if(!existsSync(archive)) {
 const response=await fetch(`https://www.cpan.org/src/5.0/perl-${version}.tar.gz`);
 if(!response.ok)throw Error(`Perl download failed: ${response.status}`);
 writeFileSync(archive,Buffer.from(await response.arrayBuffer()));
}
if(createHash('sha256').update(readFileSync(archive)).digest('hex')!==sha256)throw Error('Perl source checksum mismatch');
const sources=join(build,'perl-source');mkdirSync(sources,{recursive:true});run('tar',['-xzf',archive,'-C',sources]);
const cwd=join(sources,`perl-${version}`);
run('sh',['Configure','-des',`-Dprefix=${prefix}`,'-Dman1dir=none','-Dman3dir=none','-Uuseshrplib'],{cwd,stdio:'inherit',env:{...process.env,MACOSX_DEPLOYMENT_TARGET:process.env.MACOSX_DEPLOYMENT_TARGET||'11.0'}});
run('make',['-j4'],{cwd,stdio:'inherit',env:{...process.env,MACOSX_DEPLOYMENT_TARGET:process.env.MACOSX_DEPLOYMENT_TARGET||'11.0'}});run('make',['install'],{cwd,stdio:'inherit',env:{...process.env,MACOSX_DEPLOYMENT_TARGET:process.env.MACOSX_DEPLOYMENT_TARGET||'11.0'}});
copyFileSync(join(cwd,'Artistic'),join(root,'vendor/licenses/Perl-Artistic.txt'));
copyFileSync(join(cwd,'Copying'),join(root,'vendor/licenses/Perl-GPL-1.txt'));
for(const name of ['Perl-Artistic.txt','Perl-GPL-1.txt'])chmodSync(join(root,'vendor/licenses',name),0o644);
console.log('Private build Perl installed. Run npm run engine:deps.');
