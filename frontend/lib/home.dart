// home_screen.dart
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import 'upload.dart';
import 'package:provider/provider.dart';

import 'auth.dart';
import 'video_screen.dart';

class HomeScreen extends StatefulWidget {
  @override
  _HomeScreenState createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  @override
  Widget build(BuildContext context) {
    final auth = Provider.of<AuthService>(context);
    return Scaffold(
      appBar: AppBar(
        title: Text('Home'),
        actions: [
          IconButton(
            icon: Icon(Icons.logout),
            onPressed: () async { await auth.logout(); },
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () async {
          await Navigator.push(context, MaterialPageRoute(builder: (context) => UploadScreen()));
          setState(() {});
        },
        child: Icon(Icons.upload),
      ),
      body: FutureBuilder(
        future: auth.api("video").then((res) async {
          final files = await auth.api("video/file");
          final fMap = Map.fromIterable(files, key: (e) => e['_id'], value: (e) => e);
          return res.map((e) { e['file'] = fMap[e['file']]; return e; }).toList();
        }),
        builder: (context, snapshot) {
          if (snapshot.connectionState == ConnectionState.done) {
            if (snapshot.hasError) {
              return Text('Error: ${snapshot.error}');
            } else {
              final videos = snapshot.data;
              return ListView.builder(
                itemCount: videos.length,
                itemBuilder: (context, index) {
                  return VideoCard(video: videos[index]);
                },
              );
            }
          } else {
            return CircularProgressIndicator();
          }
        }
      )
    );
  }
}

class VideoCard extends StatefulWidget {
  final Map<String, dynamic> video;

  VideoCard({required this.video});

  @override
  _VideoCardState createState() => _VideoCardState();
}

class _VideoCardState extends State<VideoCard> {
  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: () {
        if (!kIsWeb) {
          Navigator.push(context, MaterialPageRoute(builder: (context) => VideoScreen(video: widget.video['_id'])));
        } else {
          final uri = Uri.parse("${AuthService.baseUrl}/media/${widget.video['_id']}");
          launchUrl(uri);
        }
      },
      child: ListTile(
        title: Text(widget.video['name'] ?? widget.video['_id']),
        subtitle: Text(widget.video['file']['video_codec']),
        leading: CachedNetworkImage(
          imageUrl: "${AuthService.baseUrl}/video/thumb/${widget.video['file']['_id']}",
          placeholder: (context, url) => CircularProgressIndicator(),
        ),
      )
    );
  }
}

