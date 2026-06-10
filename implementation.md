# Semantic Proxy Layer for LLMs

Required:
- Find a way to vectorize queries and store them in a database
- Find a way to perform similarity operations on new queries
- Create a procedure to vectorize outgoing queries, run similarity, and if exists in a KV store, return the associated query


Specifications:
- What data structures will work at scale?
- How will "similarity" be determined?
- What is the actual data structure associated with a KV store that will work at scale?
- What conditions do queries need to meet in order to exist in the KV store?
- How are queries added?


Workflow
--> Embed queries 
--> run cosine similarity, create vector store
--> build KV based on conditions: (latency too high? durable/binary question answer? [intent] query frequency?) Build read replicas on hot keys to mitigate traffic congestion 
--> 

File system
main.go --> handle the program flow and routing of data, organized as a pipeline
resources.go --> AWS resource definitions
db.go --> postgres handling
vectorize.go --> handle vectorization
kvroute.go --> Handle Redis utilization, KV conditions

docker: Dockerfile & docker-compose.yaml for manifest and image creation

Features:
The similarity threshold τ
--> you need a concrete starting value (0.92 is a reasonable default) and a plan for tuning it. Too low and you return wrong cached answers silently. This is the highest-risk parameter in the whole system.

Cache key design
--> when you get a vector DB hit, what's the key you use to look up the response in Redis? It needs to be stored as metadata on the pgvector row and retrieved alongside the similarity score.

TTL strategy
--> how long do cached responses live? A response cached today might be wrong in 30 days if the underlying model changes. You need expiry on both the Redis entry and a mechanism to prune stale pgvector rows.

Error handling on the LLM call
--> if Anthropic returns a 500 or a refusal, you should not admit that response to the cache. This is a write-back gate that's easy to miss.

Advanced features
* created_at timestamp in postgress to keep entries relevant.
* adaptive cosine similarity parameter: takes into account changing state of the database and recent similarity searches.
