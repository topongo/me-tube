import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'auth.dart';

class LoginScreen extends StatefulWidget {
  @override
  _LoginScreenState createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _isLoading = false;

  Future<void> _signIn(BuildContext context) async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _isLoading = true);
    try {
      // retriever AuthService from Provider
      final authService = Provider.of<AuthService>(context, listen: false);
      await authService.login(
        _usernameController.text,
        _passwordController.text,
      );
      if (context.mounted) {
        if (authService.isAuthenticated) {
          if (authService.passwordReset == true) {
            Navigator.pushReplacementNamed(context, "/password_reset");
          } else {
            Navigator.pushReplacementNamed(context, "/");
          }
        } else {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text("Login was successful, but an error occurred.")),
          );
        }
      }
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text("$e")),
      );
    } finally {
      setState(() => _isLoading = false);
    }
  }


  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('Login')),
      body: Padding(
        padding: EdgeInsets.all(20),
        child: Form(
          key: _formKey,
          child: Column(
            children: [
              TextFormField(
                controller: _usernameController,
                autofillHints: [AutofillHints.username],
                decoration: InputDecoration(labelText: 'Username'),
                keyboardType: TextInputType.text,
                validator: (value) =>
                    value!.isEmpty ? 'Enter your username' : null,
                autofocus: true,
              ),
              TextFormField(
                controller: _passwordController,
                autofillHints: [AutofillHints.password],
                decoration: InputDecoration(labelText: 'Password'),
                obscureText: true,
                validator: (value) =>
                    value!.isEmpty ? 'Enter your password' : null,
                onEditingComplete: () => _signIn(context),
              ),
              SizedBox(height: 20),
              _isLoading
                  ? CircularProgressIndicator()
                  : ElevatedButton(
                      onPressed: () => _signIn(context),
                      child: Text('Sign In'),
                    ),
              TextButton(
                onPressed: () {
                  // Add navigation to sign-up screen
                },
                child: Text('Create Account'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
