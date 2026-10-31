# Third-party software

Imapsync Studio's own code is MIT licensed. Bundled third-party components keep
their original copyrights and licenses; they are not relicensed under MIT.

## imapsync 2.314

Copyright Gilles LAMIRAL and contributors. Distributed under the No Limit Public
License. The unmodified source, full license and pinned upstream commit/checksum
are in `vendor/imapsync/` (installed under `third-party/imapsync/`).
Upstream: https://github.com/imapsync/imapsync
License: https://imapsync.lamiral.info/LICENSE

## Perl and PAR

The native engine contains a Perl interpreter and PAR runtime, together with Perl
modules and their original source/POD copyright notices. Perl and PAR are
available under the Perl licensing terms (Artistic License or GPL); full terms
are included in `vendor/licenses/PERL-PAR-LICENSE.txt`.

Perl: https://www.perl.org/
PAR::Packer: https://metacpan.org/dist/PAR-Packer

## Runtime modules and certificate data

Dependencies include Mail::IMAPClient, IO::Socket::SSL, Net::SSLeay,
Authen::SASL, Authen::NTLM, Digest::HMAC, Encode::IMAPUTF7,
File::Copy::Recursive, IO::Tee, Term::ReadKey, Unicode::String, Readonly,
Sys::MemInfo, Regexp::Common, File::Tail, Mozilla::CA and their dependencies.
They retain their original licenses and copyright notices. Module source/POD is
embedded in the PAR archive; generated build information and collected dependency
license/readme files are installed under `third-party/engine-build/`.

Mozilla CA certificate data: https://curl.se/docs/caextract.html
Mozilla Public License 2.0: `vendor/licenses/MPL-2.0.txt`.
System-provided libraries remain operating-system components. Any additionally
packed shared libraries are recorded with the engine build and retain their
upstream licenses. Packagers must review the generated dependency inventory and
notices when changing dependencies or toolchains.

## ring

Microsoft ID-token RSA signature validation uses ring 0.17.14. Its license and
third-party notices are included as `vendor/licenses/ring-LICENSE*.txt`.
Upstream: https://github.com/briansmith/ring
