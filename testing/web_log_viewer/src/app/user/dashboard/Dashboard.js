import React from 'react';
import PropTypes from 'prop-types';
import { withStyles } from '@material-ui/core/styles';
import CssBaseline from '@material-ui/core/CssBaseline';
import Typography from '@material-ui/core/Typography';
import Grid from '@material-ui/core/Grid';
import Card from '@material-ui/core/Card';
import CardActionArea from '@material-ui/core/CardActionArea';
import CardActions from '@material-ui/core/CardActions';
import CardContent from '@material-ui/core/CardContent';
import CardMedia from '@material-ui/core/CardMedia';
import Button from '@material-ui/core/Button';
import CircularProgress from '@material-ui/core/CircularProgress';
import image from '../../../images/chart.jpg';
import addChannel from '../../../images/addChannel.png';
import errorImage from '../../../images/error.png'
import { styles } from '../shared/styles';
import { connect } from 'react-redux';
import { withRouter } from 'react-router-dom';
import { getDashboardData } from './actions';
import _ from 'lodash';
import { navigation } from '../../navigation'
import { secondaryMenuContent } from './secondaryMenuContent'

class Dashboard extends React.Component {

  state = {
    isLoading: '',
    error: '',
    data: ''
  };

  componentDidMount() {
    this.props.titleHandler("Dashboard")
    this.props.secondaryMenu(secondaryMenuContent)
  }

  openChannelPage = (id, host) => {
    this.props.history.push({
      pathname: navigation.channel + '/' + id,
      state: {
        host: host,
      }
    });
  }

  openSettingsPage = (id, host) => {
    this.props.history.push({
      pathname: navigation.channel + '/' + id + '/settings',
      state: {
        host: host,
      }
    });
  }

  render() {
    const { classes } = this.props;

    return (
      <React.Fragment>
        <CssBaseline />
        <div className={classes.root}>
          <div className={classes.appBarSpacer} />
          <div styles={{ flexGrow: 1 }}>
            <Grid cols="4" container spacing={10}>
              {this.renderChannelsOrProgress()}
            </Grid>
          </div>

        </div>
      </React.Fragment>
    );
  }

  renderChannelsOrProgress() {
    const { classes } = this.props;
    if (this.props.isLoading)
      return (<CircularProgress className={classes.progress} size={100} />)
    if (this.props.error)
      return (<Card className={classes.errorCard}>
        <CardActionArea>
          <CardContent>
            <img src={errorImage} className={classes.errorImage} alt='error icon' />
            <Typography gutterBottom variant="h5" component="h2">
              Error while loading channels</Typography>
            <Typography component="p">
              {this.props.error}</Typography>
          </CardContent>
        </CardActionArea>
      </Card>)

    let data = [];
    for (let i = 0; i < localStorage.length; i = i + 1) {
      let key = localStorage.key(i);
      let value = localStorage[key];
      data.push({ key, value });
    }

    let tiles = _.map(data, function (element) {
      return {
        title: element.value,
        id: element.key
      }
    });
    let tileComponents = tiles.map(tile => this.renderSingleTile(tile))
    tileComponents.push(this.renderSinglePlaceholderTile())
    return tileComponents
  }

  renderSingleTile = (tile) => {
    const { classes } = this.props;
    return (
      <React.Fragment>
        <Grid item xs={3}>
          <Card className={classes.card}  
            onClick={() => this.openChannelPage(tile.id, tile.title)}
            >
            <CardActionArea className={classes.card}>
              <CardMedia
                component="img"
                alt={tile.title}
                className={classes.media}
                image={image}
                title={tile.title}
              />
              <CardContent>
                <Typography gutterBottom variant="h5" component="h2">
                  {tile.id} </Typography>
                <Typography component="p">
                  {tile.title}</Typography>
              </CardContent>
            </CardActionArea>
            <CardActions>
              <Button size="small" color="primary" onClick={() => this.openChannelPage(tile.id, tile.title)}>Logs</Button>
              <Button size="small" color="primary" onClick={() => this.openSettingsPage(tile.id, tile.title)}>Settings</Button>
            </CardActions>
          </Card>
        </Grid>
      </React.Fragment>
    )
  }

  renderSinglePlaceholderTile = () => {
    const { classes } = this.props;
    return (
      <Grid item xs={2}>
        <Card className={classes.card}  >
          <CardActionArea className={classes.card}>
            <CardMedia
              component="img"
              alt="add new channel img"
              className={classes.media}
              image={addChannel}
              title="Add new channel"
              onClick={() => this.openChannelPage("new")}
            />
          </CardActionArea>
        </Card>
      </Grid>
    )
  }
}

Dashboard.propTypes = {
  classes: PropTypes.object.isRequired,
  titleHandler: PropTypes.func.isRequired
};

const mapStateToProps = state => {
  console.log("dashboard")
  console.log(state)
  return {
    isLoading: state.dashboard.isLoading,
    error: state.dashboard.error,
    data: state.dashboard.data
  }
}

export default connect(mapStateToProps, { getDashboardData })(withStyles(styles)(withRouter(Dashboard)))