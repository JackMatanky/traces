#!/usr/bin/perl
my ($sig, $pid) = @ARGV;
$sig =~ s/^-//;
$pid =~ /^\d+$/ or exit 2;
$sig =~ /^\d+$/ or exit 2;
my $rc = kill($sig + 0, $pid + 0);
exit($rc ? 0 : 1);
