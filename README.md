### Dynamic PRM Library

Dynamic PRM maintains a path-finding graph for efficient path lookups.

The librarys main purpose is to evaluate alternative approaches to constructing and maintaining the graph.

The dprm structure may be initialized from a PrmConfig. In the config you can specify width, height, a RNG Seed, and a desired number of obstacles, to generate a random set of obstacles for an initial graph.

