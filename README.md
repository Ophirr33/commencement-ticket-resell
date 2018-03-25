# commencement-ticket-resell
A little website for NEU students to find other students that want to buy/sell commencement tickets

Uses Actix-Web + Diesel + Sqlite for the backend

Uses pure html+css+ajax for the frontend (I could have used a framework, but I've always wanted to try doing it this way).

Since the only data this stores are email addresses, and the number of tickets being bought and sold by each user
there's no passwords by default. A random token is generated when user's confirm that they have a husky emial address,
and a sign-in link is sent to the users that they can use.
