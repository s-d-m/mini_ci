# Introduction

This project is a mini continuous integration system tailored to my own specific needs. If it also fits yours,
or you want to develop your own and take some inspiration from this, feel free to do so.

I developed this system since I wanted a CI system fitting the following requirements:

1. as simple to use as possible.
1. simple to tweak
1. with fast interactions
1. can run tests on embedded devices attached to a PC
1. extracting specific data should simple
1. automating tasks should be simple
1. possibility to follow the progression of a test execution live

All the systems out there were I looked at were failing at least one criteria. This is not to say they are
bad, only that I wasn't willing to compromise here and decided to rather roll my own for my specific needs.

Pretty much all features which can be expected from a professional CI service are not to be found here. This
is simply because I don't need them. For example, there are no user accounts, no per-user permissions, no
email notifications when a build fails, no button to cancel or retry a build, no automatic reload of the html
page to display the latest job status.

The features it actually provides are rather basic:

1. users can post a job request and specify a few parameters:
    - the git commit id to use
    - the tests to runs
    - if the tests should be executed on real hardware, qemu or both
    - ensuring the code follow the formatting rules
    - ensuring the code follow the linters rules
1. tasks are executed and the output saved in a database
1. tasks output can be retrieved and viewed
