# Cardano Chain Follower Instances

This document details how the individual chain follower instances are organized within the Hermes runtime extension.

## Basic Principles

There are 3 primary cardano networks, Mainnet, Preprod and Preview.
We will often be syncing from TIP or Genesis, and after synchronization we will be syncing from Tip eventually.

A node follower is a reasonably heavy process, and every connection to a relay node also adds load to the relay node itself.

It is not efficient or necessary to have multiple followers syncing from the same point as the data stream is identical.

## Pool of Node followers

We will maintain a Pool of active node followers.
When a wasm module in an application subscribes to a Node followers data stream, IF there is a pre-existing node follower
which can serve its needs, then it should simply receive the events from that stream.

It will not create a new stream for every subscription.

When the last subscriber to a Node followers connection/stream unsubscribes, it will stop the connection.

This means that there is not one follower per application, there is one follower per stream type.

For example,  if a wasm module subscribes to a stream of events from Genesis, and Tip, it will have two active streams.
If a second application also subscribes to Genesis, it can retrieve the same events and there is no need to start a third follower.

## Implementation details

Application modules can subscribe to events from Tip, Genesis OR a particular block within the block chain.
There are a number of considerations which should be handled to make stream selection efficient.

There is also a case where a stream is requested to "continue" and it needs special handling to be efficient.

### Maximizing the number of listeners to an event stream

We want to maximize the number of listeners to individual event streams.
This will improve efficiency overall.

If two applications both request a stream of blocks from mainnet, from Genesis.
Then ideally they both listen to the same stream.
However, both applications can start asynchronously,
and there is no guarantee when each application or module will subscribe to the stream.

This can cause the following situation (These event all happen in rapid succession in this example):

1. App 1 subscribes to genesis.
2. Connection to Node occurs, and block 1 is received, and sent to App 1.
3. App 2 subscribes to genesis.
4. The first connection can not be used because we are already past genesis on that stream, so we need to create a second stream.

The problem here is we have two streams, but they are only 1 block apart.  
Further, based on timing it may be that either exceeds the other depending on how fast their individual connection to the relay is.
This is not ideal.

There are a number of ways this can be solved.

1. Use a block cache for recent blocks, so that subscriptions that are close in time can use the same stream by
   re-using blocks from the cache.
2. Delay when we connect a stream to allow for the maximum number of subscribers.
3. Optimizing the streams on the fly.
    * For example if 2 streams are at different points, but they could catch up and become equal.
    the moment they become equal one of the streams is closed and all block data is now taken from a single stream.
    * This is likely to occur, for example a follower from Tip and Genesis.
    Once the Genesis follower has reached tip we now have two followers from Tip when only 1 is required.
4. An ideal solution may need multiple techniques.

*Note: There may be other techniques and these are just some obvious solutions to the issue.*

For the initial implementation of the Follower, we will employ technique 2 and a limited version of Technique 3 ONLY.

#### Delaying starting of the streams

When a stream is subscribed, and it results in a new stream, then the connection to that relay node will not happen for 5 seconds.
Any further subscriptions to the same stream can then be added to the pending connection.
Once the connection is made, then further subscriptions will only be added to it if they are valid for that connection.
Otherwise a second connection is established, again with its own 5 second start delay.

This delay occurs regardless of where the stream is to start from Tip, Genesis or a particular block.

#### Optimizing the streams

If we have two followers for a particular network, and they are BOTH on TIP,
then one is stopped and all subscribers for it are moved to the other.

This needs to check that not only are they both reported to be on TIP, but that the TIPs are identical.

We do not need to do any other stream optimization at this time.

The purpose of this is to ensure that once followers have fully synced we are not running excessive followers
that are doing the same work and reporting the same events.

### Handling "Continue" connections

There is an option to listen to a stream from "Continue" but what does "Continue" actually mean?
If there are multiple streams it could match on, which one does it match?

We will use the rule that if "Continue" is specified, it means "Follow the blockchain with the earliest blocks".

So if there are three followers, one from Genesis, one from block 10,000 and one from Tip.  
"Continue" will join the Genesis follower.

There is a further condition for "Continue" which is what happens if "Continue" is specified but there are no
blockchain followers running or pending for the same Application?
This can only happen either at the start of world for a particular Application OR
an Application has unsubscribed from all blockchain events.

In this case the "Continue" subscription should be added to list of continue subscriptions for each application.
When a block is received and about to be sent as an event to an application, we can check the "Continue" list for that app,

If there are subscriptions in that list,
and the event is from the oldest active follower for the app then they are removed from that list,
and added as active subscriptions to the selected follower.

Then all subscribed modules in the app (including any recently added from the continue list) are sent the event.

## Further Optimizations

The initial set of connection optimizations are detailed here.
Further connection optimization work will only be conducted after these are implemented and the characteristics of the
followers running in a Hermes environment are better understood.
