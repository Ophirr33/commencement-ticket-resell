# commencement-ticket-resell
A little website for NEU students to find other students that want to buy/sell commencement tickets

Uses Actix-Web + Diesel + Sqlite for the backend

Uses pure html+css+ajax for the fronten. I could have used a framework, but I've always wanted to try doing it this way. Surprisingly, it wasn't all that bad!

This only stores the users' email addresses and the number of tickets they're buying/sell. As such,
there are no passwords. A random token is generated when users confirm that they have a husky email address,
and a sign-in link is sent to the users that they can use.
