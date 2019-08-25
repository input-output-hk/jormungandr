import React from 'react';
import PropTypes from 'prop-types';
import withStyles from '@material-ui/core/styles/withStyles';
import CssBaseline from '@material-ui/core/CssBaseline';
import Paper from '@material-ui/core/Paper';
import Grid from '@material-ui/core/Grid';
import TextField from '@material-ui/core/TextField';
import { styles } from './styles'
import { navigation } from '../../../navigation'
import { withRouter } from 'react-router-dom';
import { createChannel, reset } from './actions'
import { connect } from 'react-redux';
import Typography from '@material-ui/core/Typography';
import ChannelConfirmationLoadingPage from './confirmation/loadingConfirmation/ChannelConfirmationLoadingPage';
import ChannelErrorPage from './confirmation/ChannelErrorPage';
import Button from '@material-ui/core/Button';
import ChannelConfirmationPage from './confirmation/ChannelConfirmationPage';

class ChannelSetupPage extends React.Component {

  state = {
    isLoading: false,
    error: '',
    data: '',
    isSuccess: false,
  };


  componentDidMount() {
    this.props.titleHandler("Create new Channel")
    this.props.reset();
  }

  onSubmit = (e) => {
    e.preventDefault();
    this.props.createChannel(this.state.host, this.state.name)
  }

  onChannelNameChange = (newName) => {
    this.setState({ name: newName })
  }

  onChannelHostChange = (newHost) => {
    this.setState({ host: newHost })
  }

  reset = () => {
    this.props.reset({
      data: {
        host: '',
        name: ''
      }
    })
  }

  openChannelPage = () => {
    reset();
    let name = this.props.data.data.name;
    let host = this.props.data.data.host;
    localStorage.setItem(name, host);
    this.props.history.push({
      pathname: navigation.dashboard,
    });
  }

  render() {
    const { classes } = this.props;
    console.log(this.props);
    if (this.props.isLoading) {
      return (
        <React.Fragment>
          <CssBaseline />
          <main className={classes.layout}>
            <Paper className={classes.paper}>
              <React.Fragment>
                <ChannelConfirmationLoadingPage />
              </React.Fragment>
            </Paper>
          </main>
        </React.Fragment>
      );
    }

    if (this.props.error) {
      console.log("this_error")
      return (
        <React.Fragment>
          <CssBaseline />
          <main className={classes.layout}>
            <Paper className={classes.paper}>
              <React.Fragment>
                <ChannelErrorPage data={this.state} error={this.props.error} resetAction={this.reset} />
              </React.Fragment>
            </Paper>
          </main>
        </React.Fragment>
      );
    }

    if (this.props.isSuccess) {
      console.log("this_ok")
      return (
        <React.Fragment>
          <CssBaseline />
          <main className={classes.layout}>
            <Paper className={classes.paper}>
              <React.Fragment>
                <ChannelConfirmationPage data={this.props.data} successAction={this.openChannelPage} />
              </React.Fragment>
            </Paper>
          </main>
        </React.Fragment>
      );
    }

    return (
      <React.Fragment>
        <CssBaseline />
        <main className={classes.layout}>
          <Paper className={classes.paper}>
            <React.Fragment>
              <form onSubmit={this.onSubmit.bind(this)}>
                <Typography variant="h6" gutterBottom>
                  Node Details
                </Typography>
                <Grid container spacing={24}>
                  <Grid item xs={12} sm={6}>
                    <TextField
                      required
                      ref="name"
                      id="name"
                      name="name"
                      label="name"
                      fullWidth
                      onChange={(e) => this.onChannelNameChange(e.target.value)}
                      autoComplete="fname"
                    />
                    <TextField
                      required
                      ref="host"
                      id="host"
                      name="host"
                      label="host in format: {http:://host:port}"
                      fullWidth
                      onChange={(e) => this.onChannelHostChange(e.target.value)}
                      autoComplete="fname"
                    />
                  </Grid>
                </Grid>
                <div className={classes.buttons}>
                  <Button
                    type="submit"
                    variant="contained"
                    color="primary"
                    className={classes.button}
                  >Create and Verify
                        </Button>
                </div>
              </form>
            </React.Fragment>
          </Paper>
        </main>
      </React.Fragment>
    );
  }
}

const mapStateToProps = state => {
  console.log("map")
  console.log(state)
  return {
    error: state.newChannel.error,
    isLoading: state.newChannel.isLoading,
    data: state.newChannel.data,
    isSuccess: state.newChannel.isSuccess

  }
}

ChannelSetupPage.propTypes = {
  classes: PropTypes.object.isRequired,
};

export default connect(mapStateToProps, { createChannel, reset })(withStyles(styles)(withRouter(ChannelSetupPage)));