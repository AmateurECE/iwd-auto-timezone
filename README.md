# Automatically Change Timezones on Systems running iwd

Many distributions provide solutions for this problem that are compatible with
NetworkManager, but these solutions cannot translate to systems that run `iwd`.

This daemon listens for the connection state of network interfaces from `iwd`
on the D-Bus, and then uses the free Geo-IP service provided by IPAPI to change
the system timezone.
