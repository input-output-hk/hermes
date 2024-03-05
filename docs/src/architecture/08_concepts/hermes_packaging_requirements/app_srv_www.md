# HTTP Served Application Data

## Hermes HTTP gateway

Hermes provides a complete HTTP gateway.
Applications are deployed to a sub-domain under the main domain of the hermes node.

For example, if the Hermes node is located on the machine `hermes.local`,
it will serve the application `athena` on `athena.hermes.local`.

The gateway will direct all requests to the `/api` path on the applications URL to Webasm Component Modules.
These will be delivered as Hermes events, and the Webasm Components will use HTTP gateway API's to produce responses.

## HTTP Static Data

Any application run in a browser consists of a large number of static assets.
Given the above example, the `athena` application can serve these static files contained in the Application from `/`.

These are contained in `/srv/www` within the application package.
If a file is requested that does not exist, the Hermes HTTP gateway will automatically respond with 404.
It will also respond with 404 is there is no static HTTP data in the application.
An example of an application which would not have static HTTP data is one where all data is served through the `/api` path.

These static files are mapped 1:1 from the application package to the http url path.

## Example file to url mapping

For example the following files in the package can be retrieved at the example URL.

| File | URL |
| --- | --- |
| `/srv/www/index.html` | `http://athena.hermes.local/index.html` |
| `/srv/www/icons/athena.svg` | `http://athena.hermes.local/icons/athena.svg` |

These files are served with no interaction with the Webasm component modules within the application.
Any file contained in the apps `/srv/www` directory are transparently available.

The only restriction is that the path `/api` can not be used.
This is because `/api` is reserved for http requests that are directed to Hermes Webasm component modules for service,
rather than serving a static file.

## HTTP Static data validity checks

If an application is attempted to be packaged with files in `/srv/www/api` then the packaging attempt will fail.
Further, a Hermes application with data under `/srv/www/api` will also fail to load as an invalid package.
