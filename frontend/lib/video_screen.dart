import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:provider/provider.dart';
import 'auth.dart';

class VideoScreen extends StatefulWidget {
  final String video;
  VideoScreen({required this.video});

  @override
  _VideoScreenState createState() => _VideoScreenState();
}

class _VideoScreenState extends State<VideoScreen> {
  Media? media;
  late final dynamic video;
  late final dynamic game;
  late bool liked;
  late int likes;
  late final player = Player();
  late final controller = VideoController(player);
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    final auth = Provider.of<AuthService>(context, listen: false);
    _loadVideo(auth);
  }

  Future<void> _loadVideo(AuthService auth) async {
    video = await auth.api("video/${widget.video}");
    game = await auth.api("game/${video["game"]}");
    liked = await auth.api("like/${video["_id"]}");
    likes = await auth.api("video/${video["_id"]}/likes");
    media = await auth.getVideo(video["_id"]); 
    player.open(media!);
    setState(() => _loaded = true);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: !_loaded ? Text("Video") : ListTile(
          title: Text(video["name"] ?? video["_id"]),
          subtitle: Text("${game["name"]} - ${video["owner"]}")
        ),
        elevation: 0,
        backgroundColor: Colors.transparent,
        actions: [
          !_loaded ? Container() : Row(
            children: [
              Text("$likes"),
              IconButton(
                icon: Icon(liked ? Icons.favorite : Icons.favorite_border),
                onPressed: () async {
                  final auth = Provider.of<AuthService>(context, listen: false);
                  try {
                    await auth.api("video/${video["_id"]}/like", method: liked ? "DELETE" : "POST");
                    if (context.mounted) {
                      setState(()  { 
                        liked = !liked;
                        likes += liked ? 1 : -1;
                      });
                    }
                  } catch (e) {
                    if (context.mounted) {
                      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Error while liking video: $e')));
                    }
                  }
                }
              ),
            ]
          )
        ]
      ),
      extendBodyBehindAppBar: true,
      extendBody: true,
      body: LayoutBuilder(
        builder: (context, constraints) {
          return !_loaded ? const Center(child: CircularProgressIndicator()) : Video(controller: controller);
        }
      )
    );
  }

  @override
  dispose() {
    player.dispose();
    super.dispose();
  }
}
