# The Rust poller reaches these over JNI, which R8 has no way of seeing.
-keep class com.altis.app.BackgroundPoller { *; }
-keep class com.altis.app.PollService { *; }
-keep class com.altis.app.BootReceiver { *; }
-keep class com.altis.app.PollWorker { *; }
