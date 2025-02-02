import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'auth.dart';

class UploadScreen extends StatefulWidget {
  @override
  _UploadScreenState createState() => _UploadScreenState();
}

class _UploadScreenState extends State<UploadScreen> {
  FilePickerResult? _files;
  final Map<String, String> _names = {};
  final _formKey = GlobalKey<FormState>();
  Map<String, String>? _games;
  String? _game;
  bool _uploading = false;
  double? _progress;

  @override
  initState() {
    super.initState();
    final auth = Provider.of<AuthService>(context, listen: false);
    auth.api("game").then((value) {
      setState(() {
        _games = {};
        for(final game in value) {
          _games![game['_id']] = game['name'];
        }
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Upload'),
      ),
      body: Center(
        child: Form(
          key: _formKey,
          child: Column(
            children: [
              _games == null ? const CircularProgressIndicator() : Padding(padding: EdgeInsets.only(left: 20, right: 20), child: DropdownButtonFormField(
                items: _games!.entries.map((entry) => DropdownMenuItem(value: entry.key, child: Text(entry.value))).toList(),
                onChanged: (value) => setState(() => _game = value),
                decoration: InputDecoration(labelText: 'Game'),
                validator: (value) => value == null ? "Select a Game" : null,
              )),
              ListView.builder(
                shrinkWrap: true,
                itemCount: _files == null ? 0 : _files!.files.length + 1,
                itemBuilder: (context, index) {
                  if (index == 0) {
                    return ListTile(title: const Text("Selected Files"));
                  } else {
                    index -= 1;
                  }
                  final file = _files!.files[index];
                  return ListTile(
                    leading: Icon(Icons.play_arrow),
                    title: TextField(
                      controller: TextEditingController(text: _names[file.name] ?? file.name),
                      onChanged: (value) { _names[file.name] = value; print(_names); },
                      onEditingComplete: () => setState(() {}),
                      onTapOutside: (_) => setState(() {}),
                    ),
                    subtitle: _names[file.name] == null ? null : Text('Original name: ${file.name}'),
                    trailing: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        IconButton(
                          icon: Icon(Icons.delete),
                          onPressed: () {
                            setState(() {
                              _files!.files.removeAt(index);
                              _names.remove(file.name);
                            });
                          },
                        )
                      ]
                    )
                  );
                }
              ),
              ElevatedButton(
                onPressed: () {
                  FilePicker.platform.pickFiles(allowMultiple: true, withReadStream: true).then((result) {
                    if (result != null) {
                      setState(() {
                        if (_files == null) {
                          _files = result;
                        } else {
                          _files!.files.addAll(result.files);
                        }
                      });
                    }
                  });
                }, 
                child: Text(_files == null ? 'Select Files' : 'Add Files')
              ),
              SizedBox(height: 50),
              // submit
              ElevatedButton(
                onPressed: () => _upload(context),
                child: !_uploading ? const Text("Upload") : const CircularProgressIndicator(),
              ),
              _progress == null ? SizedBox(height: 0) : Padding(
                padding: EdgeInsets.only(left: 20, right: 20),
                child: Column(
                  children: [
                    Text("Progress: ${(_progress! * 100).toStringAsFixed(1)}%"),
                    LinearProgressIndicator(
                      value: _progress!,
                    )
                  ]
                )
              )
            ]
          )
        )
      )
    );
  }

  Future<void> _upload(context) async {
    if (!_formKey.currentState!.validate()) return;
    if (_files == null || _files!.files.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: const Text("Select at least one file."))
      );
      return;
    }

    setState(() => _uploading = true);
    try {
      final auth = Provider.of<AuthService>(context, listen: false);
      final response = await auth.uploadVideos(
      _game!,
        _files!.files, 
        _names,
        (bytes, totalBytes) {
          _progress = bytes / totalBytes;
        }
      );
      print(response);
    } finally {
      setState(() => _uploading = false);
    }
  }
}
