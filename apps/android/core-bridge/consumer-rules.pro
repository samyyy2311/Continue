# Keep JNA symbols and UniFFI generated classes
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { *; }
-keep class org.continue.bridge.** { *; }
