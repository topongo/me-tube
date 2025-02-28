import 'dart:async';
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
  final Map<String, bool> _publics = {};
  final _formKey = GlobalKey<FormState>();
  Map<String, String>? _games;
  String? _game;
  bool _uploading = false;
  double? _progress;

  @override
  initState() {
    super.initState();
    final auth = Provider.of<AuthService>(context, listen: false);
    auth.api("game/user/${auth.username}").then((value) {
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
              SizedBox(height: 40),
              Text("Files to be uploaded", style: Theme.of(context).textTheme.titleLarge),
              Padding(
                padding: EdgeInsets.only(top: 20, bottom: 20),
                child: _files == null ? const Text("No files selected") : ListView.builder(
                  shrinkWrap: true,
                  itemCount: _files == null ? 0 : _files!.files.length,
                  itemBuilder: (context, index) {
                    final file = _files!.files[index];
                    return ListTile(
                      leading: Icon(Icons.play_arrow),
                      title: TextField(
                        controller: TextEditingController(text: _names[file.name] ?? file.name),
                        onChanged: (value) { _names[file.name] = value; },
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
                                _publics.remove(file.name);
                              });
                            },
                          ),
                          IconButton(
                            icon: Icon(_publics[file.name]! ? Icons.public : Icons.lock),
                            onPressed: () {
                              setState(() {
                                // print("Toggling public for ${file.name} => ${_publics[file.name]}");
                                _publics[file.name] = !_publics[file.name]!;
                              });
                            },
                          )
                        ]
                      )
                    );
                  }
                ),
              ),
              ElevatedButton(
                onPressed: () async {
                  final result = await FilePicker.platform.pickFiles(allowMultiple: true, withReadStream: true, withData: false);
                  debugPrint("Selected files: $result");
                  if (result != null) {
                    setState(() {
                      if (_files == null) {
                        _files = result;
                        for (final f in result.files) {
                          _publics[f.name] = false;
                        }
                      } else {
                        _files!.files.addAll(result.files);
                        for (final f in result.files) {
                          _publics[f.name] = false;
                        }
                      }
                    });
                  }
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
    if (_uploading) return;
    if (_files == null || _files!.files.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: const Text("Select at least one file."))
      );
      return;
    }

    setState(() { _uploading = true; _progress = 0; });
    try {
      final auth = Provider.of<AuthService>(context, listen: false);
      int prev = 0;
      final List<dynamic> response = await auth.uploadVideos(
      _game!,
        _files!.files, 
        _names,
        _publics,
        (bytes, totalBytes) {
          // print("Progress: $bytes / $totalBytes");
          if (bytes - prev > 1024 * 1024) {
            setState(() { _progress = bytes / totalBytes; });
            prev = bytes;
          }
        }
      );
      Navigator.pop(context, response);
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text("Upload error: $e"))
      );
      _files = null;
    } finally {
      setState(() { _uploading = false; _progress = null; });
    }
  }
}

