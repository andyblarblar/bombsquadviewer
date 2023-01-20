# Bombsquad remote camera viewing application

This uses uvc to connect to a camera on the server, then sends that over the network using TCP, finally getting consumed
into an opencv mat and imshowed.

# Note

This application has crazy delay in the IAVS over wifi, but it gets very low lag when the le-potato is hardwired and the client
is connected over 5Ghz.

# Branches

Downsample - Custom UDP protocol, super slow but neat to look at
TCP - TCP based communication, very fast and reliable