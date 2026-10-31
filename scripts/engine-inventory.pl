use strict;
use warnings;
use Archive::Zip;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
use File::Temp qw(tempdir);
use File::Basename qw(basename);
my ($binary)=@ARGV;
my $zip=Archive::Zip->new();
$zip->read($binary)==Archive::Zip::AZ_OK() or die "Cannot read PAR archive";
my $temp=tempdir(CLEANUP=>1);
my @members=$zip->members;
my %names=map {basename($_->fileName)=>1} @members;
my @inventory;
for my $member (@members) {
 next if $member->isDirectory;
 my $name=$member->fileName;
 push @inventory,{path=>$name,sha256=>sha256_hex($member->contents)};
 if ($^O eq 'darwin' && $name=~/\.(?:bundle|dylib)$/) {
  my $path="$temp/".scalar(@inventory)."-".basename($name);
  $member->extractToFileNamed($path)==Archive::Zip::AZ_OK() or die "Cannot inspect $name";
  open my $fh,'-|','/usr/bin/otool','-L',$path or die $!;
  <$fh>;
  while (my $line=<$fh>) {
   $line=~s/^\s+|\s+\(.*$|\s+$//g;
   next if $line=~m{^/(?:usr/lib/|System/Library/)};
   next if $line=~m{^\@loader_path/([^/]+)$} && $names{$1};
   die "Non-portable native dependency in $name: $line\n";
  }
  close $fh;
 }
}
print JSON::PP->new->canonical->pretty->encode(\@inventory);
