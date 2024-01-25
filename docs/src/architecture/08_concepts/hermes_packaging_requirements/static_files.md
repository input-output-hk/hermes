# Packaging Static Files in a Hermes Application

TODO

** SJ Notes **

There are two kinds of static files.
1. Static files which will be served directly by the applications http gateway (built in behaviour of hermes).
   1. These files would be sourced from the path `/` but appear in their own sub-directory in the HDF5 file.
   2. Maybe to simplify comprehension we could put these files under `/var/www` so its similar to a normal web server.
2. Static files which ONLY can be read by Webasm modules in the Application.
   1. These can be anything.
   2. Maybe to simplify comprehension we could put these files under `/var/data` so its similar to the served files, but a distinct set.
  
The file location is just a suggestion, so feel free to suggest an alternative.
