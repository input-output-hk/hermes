import 'dart:async';
import 'dart:developer';

import 'package:bloc/bloc.dart';
import 'package:dashboard/app_bloc_observer.dart';
import 'package:flutter/widgets.dart';

Future<void> bootstrap(FutureOr<Widget> Function() builder) async {
  WidgetsFlutterBinding.ensureInitialized();

  FlutterError.onError = (details) {
    log(
      details.exceptionAsString(),
      stackTrace: details.stack,
    );
  };

  Bloc.observer = AppBlocObserver();

  runApp(await builder());
}
